#!/usr/bin/env python3
"""Compile/run incremental Rust producers and validate output with libxml2.

Run: uv run --with lxml python scripts/verify_incremental_writer.py
Requires the built debug CLI and GNU time. Each RSS sample is a fresh process.
An existing .venv with lxml can run this script directly.
"""

import argparse
import json
import platform
import re
import subprocess
import tempfile
from pathlib import Path

from lxml import etree

REPO = Path(__file__).resolve().parents[1]
SCHEMA = REPO / "tests/fixtures/incremental/items.xsd"


def run(args, cwd=REPO):
    result = subprocess.run(
        list(map(str, args)), cwd=cwd, capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise RuntimeError(
            f"{' '.join(map(str, args))}\n{result.stdout}\n{result.stderr}"
        )
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output", type=Path, default=REPO / "target/incremental-rss.json"
    )
    parser.add_argument(
        "--counts", type=int, nargs="+", default=[1000, 100000, 1000000]
    )
    parser.add_argument("--repeats", type=int, default=3)
    args = parser.parse_args()
    report = {
        "commit": run(["git", "rev-parse", "HEAD"]).strip(),
        "working_tree": run(["git", "diff", "--stat"]),
        "platform": platform.platform(),
        "cpu": next(
            (
                line.split(":", 1)[1].strip()
                for line in Path("/proc/cpuinfo").read_text().splitlines()
                if line.startswith("model name")
            ),
            platform.processor(),
        ),
        "rustc": run(["rustc", "--version"]).strip(),
        "profile": "debug (RSS only; no throughput claim)",
        "schema": str(SCHEMA.relative_to(REPO)),
        "samples": [],
    }
    with tempfile.TemporaryDirectory(prefix="polyxml-incremental-") as temporary:
        folder = Path(temporary)
        for owned in [False, True]:
            project = folder / str(owned)
            source = project / "src"
            source.mkdir(parents=True)
            for module, xsd in [
                (
                    "scalar",
                    '<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:complexType name="NumbersType"><xs:sequence><xs:element name="Number" type="xs:int" minOccurs="1" maxOccurs="3"/></xs:sequence></xs:complexType><xs:element name="Numbers" type="NumbersType"/></xs:schema>',
                ),
                ("items", SCHEMA.read_text()),
                (
                    "bounded",
                    SCHEMA.read_text().replace(
                        'maxOccurs="unbounded"', 'maxOccurs="2"'
                    ),
                ),
                (
                    "optional",
                    SCHEMA.read_text().replace('minOccurs="1"', 'minOccurs="0"'),
                ),
                (
                    "plain",
                    SCHEMA.read_text()
                    .replace(
                        ' xmlns:t="urn:items" targetNamespace="urn:items" elementFormDefault="qualified"',
                        "",
                    )
                    .replace("t:", ""),
                ),
            ]:
                schema_file = project / f"{module}.xsd"
                schema_file.write_text(xsd)
                run(
                    [
                        REPO / "target/debug/polyxml",
                        "generate",
                        schema_file,
                        "--lang",
                        "rust",
                        "--out",
                        source,
                        "--zero-copy",
                        str(not owned).lower(),
                    ]
                )
            cargo = (REPO / "crates/polyxml-core/Cargo.toml").read_text()
            version = re.search(
                r'quick-xml\s*=\s*(?:\{\s*version\s*=\s*)?"([^"]+)"', cargo
            ).group(1)
            (project / "Cargo.toml").write_text(f'''[package]
name="incremental-producer-{str(owned).lower()}"
version="0.0.0"
edition="2021"
[dependencies]
polyxml={{path={json.dumps(str(REPO / "crates/polyxml-core"))}}}
quick-xml="{version}"
serde={{version="1",features=["derive"]}}
serde_json="1"
''')
            (
                source / "main.rs"
            ).write_text("""mod items; mod bounded; mod optional; mod plain; mod scalar;
use std::io::{BufWriter, Write};
fn main() {
    // Both lower and upper occurrence limits must fail at runtime.
    assert!(items::Batch::write_batch_items(&mut std::io::sink(), std::iter::empty()).is_err());
    assert!(bounded::Batch::write_batch_items(&mut std::io::sink(), (0..3).map(|id| bounded::ItemType { name: "one".into(), quantity: 1, id })).is_err());
    optional::Batch::write_batch_items(&mut std::io::sink(), std::iter::empty()).unwrap();
    let mut scalar_xml = Vec::new();
    scalar::Numbers::write_numbers_items(&mut scalar_xml, -1..2).unwrap();
    std::fs::write("scalar.xml", scalar_xml).unwrap();
    let mut plain_xml = Vec::new();
    plain::Batch::write_batch_items(&mut plain_xml, [plain::ItemType { name: "plain".into(), quantity: 1, id: 0 }]).unwrap();
    std::fs::write("plain.xml", plain_xml).unwrap();
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> { Err(std::io::Error::other("broken sink")) }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }
    let mut consumed = 0;
    assert!(items::Batch::write_batch_items(&mut Broken, (0..3).map(|id| { consumed += 1; items::ItemType { name: "one".into(), quantity: 1, id } })).is_err());
    assert_eq!(consumed, 0); // Opening write fails before asking for an item.
    struct Limited { remaining: usize }
    impl Write for Limited {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            if self.remaining == 0 { return Err(std::io::Error::other("full sink")); }
            let n = self.remaining.min(data.len()); self.remaining -= n; Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }
    let mut produced_before_failure = 0;
    assert!(items::Batch::write_batch_items(&mut Limited { remaining: 100 }, (0..10).map(|id| {
        produced_before_failure += 1;
        items::ItemType { name: "A & <B>".into(), quantity: 7, id }
    })).is_err());
    assert!((1..10).contains(&produced_before_failure));
    let count: i32 = std::env::args().nth(1).unwrap().parse().unwrap();
    struct Observed<W> { inner: W, bytes: std::rc::Rc<std::cell::Cell<usize>> }
    impl<W: Write> Write for Observed<W> {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            let n = self.inner.write(data)?;
            self.bytes.set(self.bytes.get() + n);
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> { self.inner.flush() }
    }
    let bytes = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut file = Observed { inner: BufWriter::new(std::fs::File::create("output.xml").unwrap()), bytes: bytes.clone() };
    let mut produced = 0;
    let mut previous_bytes = 0;
    items::Batch::write_batch_items(&mut file, (0..count).map(|id| {
        assert!(bytes.get() > previous_bytes, "previous item must be written before requesting the next");
        previous_bytes = bytes.get();
        produced += 1;
        items::ItemType { name: "A & <B>".into(), quantity: 7, id }
    })).unwrap();
    assert_eq!(produced, count);
    file.flush().unwrap();
}
""")
            target = REPO / "target/incremental-writer"
            run(["cargo", "build", "--offline", "--target-dir", target], project)
            binary = target / "debug" / f"incremental-producer-{str(owned).lower()}"
            validator = etree.XMLSchema(etree.parse(str(SCHEMA)))
            scalar_validator = etree.XMLSchema(etree.parse(str(project / "scalar.xsd")))
            plain_validator = etree.XMLSchema(etree.parse(str(project / "plain.xsd")))
            for count in args.counts:
                for repeat in range(args.repeats):
                    run(
                        [
                            "/usr/bin/time",
                            "-f",
                            "%M",
                            "-o",
                            project / "rss.txt",
                            binary,
                            count,
                        ],
                        project,
                    )
                    observed = 0
                    for _, element in etree.iterparse(
                        str(project / "output.xml"),
                        events=("end",),
                        tag="{urn:items}Item",
                        schema=validator,
                    ):
                        assert element.findtext("{urn:items}Name") == "A & <B>"
                        assert element.findtext("{urn:items}Quantity") == "7"
                        assert int(element.get("id")) == observed
                        observed += 1
                        element.clear()
                        while element.getprevious() is not None:
                            del element.getparent()[0]
                    assert observed == count
                    scalar_validator.assertValid(
                        etree.parse(str(project / "scalar.xml"))
                    )
                    plain_validator.assertValid(etree.parse(str(project / "plain.xml")))
                    sample = {
                        "owned": owned,
                        "items": count,
                        "repeat": repeat,
                        "peak_rss_kib": int((project / "rss.txt").read_text()),
                        "output_bytes": (project / "output.xml").stat().st_size,
                        "xsd_valid": True,
                    }
                    report["samples"].append(sample)
                    print(json.dumps(sample), flush=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
