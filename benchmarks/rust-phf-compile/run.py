#!/usr/bin/env python3
"""Measure generated consumer builds sequentially, under a mandatory cgroup cap."""

import argparse
import importlib.util
import json
import os
import platform
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SUITE = Path(__file__).resolve().parent


def run(args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tiers", type=int, nargs="+", default=[600, 1500])
    parser.add_argument(
        "--field-type", choices=["unsignedInt", "string"], default="unsignedInt"
    )
    parser.add_argument(
        "--strategies", nargs="+", choices=["match", "phf"], default=["match", "phf"]
    )
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--output", type=Path, required=True)
    options = parser.parse_args()
    if options.repeats < 1 or any(n < 1 for n in options.tiers):
        parser.error("tiers and repeats must be positive")
    if (options.output / "results.json").exists():
        parser.error("choose a fresh output directory to preserve previous results")
    for count in options.tiers:
        for strategy in options.strategies:
            build_dir = (
                SUITE / "target" / options.field_type / str(count) / strategy / "target"
            )
            if build_dir.exists():
                parser.error(f"cold build requires a fresh target: remove {build_dir}")
    env = dict(os.environ)
    # Ignore inherited bypass/nesting flags: each build gets its own hard cap.
    for key in ("POLYXML_MEMCAP_DISABLE", "POLYXML_MEMCAP_LEVEL", "CARGO_TARGET_DIR"):
        env.pop(key, None)
    env.update(POLYXML_MEMCAP_BACKEND="systemd", CARGO_BUILD_JOBS="1")
    env.setdefault("POLYXML_MEMCAP_PCT", "40")
    cap = [str(ROOT / "scripts/memcap.sh")]
    run(cap + ["true"], env=env)
    cli = Path(env.get("POLYXML_BIN", ROOT / "target/debug/polyxml"))
    spec = importlib.util.spec_from_file_location(
        "fixtures", ROOT / "scripts/gen_tag_dispatch_fixtures.py"
    )
    fixtures = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fixtures)
    options.output.mkdir(parents=True, exist_ok=True)
    metadata = {
        "revision": run(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True
        ).stdout.strip(),
        "rustc": run(["rustc", "-Vv"], capture_output=True).stdout,
        "os": platform.platform(),
        "cpu": Path("/proc/cpuinfo")
        .read_text()
        .split("model name\t: ")[1]
        .splitlines()[0],
        "meminfo": Path("/proc/meminfo").read_text(),
        "memcap_pct": env["POLYXML_MEMCAP_PCT"],
        "jobs": 1,
        "field_type": options.field_type,
        "rustflags": env.get("RUSTFLAGS", ""),
        "encoded_rustflags": env.get("CARGO_ENCODED_RUSTFLAGS", ""),
        "results": [],
    }
    for count in options.tiers:
        tags = fixtures.make_tags(count, fixtures.SEED + count)
        folder = SUITE / "target" / options.field_type / str(count)
        folder.mkdir(parents=True, exist_ok=True)
        schema = folder / "record.xsd"
        schema.write_text(
            '<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">'
            '<xs:element name="Record" type="RecordType"/>'
            '<xs:complexType name="RecordType"><xs:sequence>'
            + "".join(
                f'<xs:element name="{tag}" type="xs:{options.field_type}"/>'
                for tag in tags
            )
            + "</xs:sequence></xs:complexType></xs:schema>"
        )
        xml = (
            "<Record>"
            + "".join(
                f"<{tag}>{i if options.field_type == 'unsignedInt' else f'value-{i}'}</{tag}>"
                for i, tag in enumerate(tags)
            )
            + "</Record>"
        )
        for strategy in options.strategies:
            crate = folder / strategy
            src = crate / "src"
            src.mkdir(parents=True, exist_ok=True)
            flags = ["--feature", "phf"] if strategy == "phf" else []
            run(
                cap
                + [
                    str(cli),
                    "generate",
                    str(schema),
                    "--lang",
                    "rust",
                    "--out",
                    str(src),
                ]
                + flags,
                env=env,
            )
            (crate / "Cargo.toml").write_text(
                '[package]\nname="dispatch-consumer"\nversion="0.0.0"\nedition="2021"\n'
                "[workspace]\n[dependencies]\n"
                f'polyxml={{path="{ROOT / "crates/polyxml-core"}"}}\n'
                'quick-xml="0.42"\nserde={version="1",features=["derive"]}\nserde_json="1"\n'
                + ('phf="0.14"\n' if strategy == "phf" else "")
            )
            (crate / "record.xml").write_text(xml)
            # Exercise both codecs and every field; use a runtime input to keep
            # LLVM from specializing away the decoder or its dispatch table.
            (src / "main.rs").write_text(
                "mod record;\nfn main() {\n"
                "let xml=std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();\n"
                "let value=record::Record::from_xml(&xml).unwrap();\n"
                "let mut output=Vec::new();\n"
                "let mut writer=quick_xml::Writer::new(&mut output);\n"
                'value.encode_xml(&mut writer, Some("Record")).unwrap();\n'
                "assert_eq!(output, xml.as_bytes());\nstd::hint::black_box(value);\n}\n"
            )
            log = options.output / f"{count}-{strategy}.log"
            result = {"fields": count, "strategy": strategy, "builds": []}
            metadata["results"].append(result)
            build_dir = crate / "target"
            if build_dir.exists():
                parser.error(f"cold build requires a fresh target: remove {build_dir}")
            with log.open("w") as stream:
                # Download/resolve outside timed cold builds; retain exact lockfile.
                run(
                    cap
                    + ["cargo", "fetch", "--manifest-path", str(crate / "Cargo.toml")],
                    env=env,
                    stdout=stream,
                    stderr=stream,
                )
                (options.output / f"{count}-{strategy}.lock").write_text(
                    (crate / "Cargo.lock").read_text()
                )
                for repeat in range(options.repeats + 1):
                    if repeat:
                        (src / "record.rs").touch()
                    phase = "cold" if repeat == 0 else f"rebuild-{repeat}"
                    print(f"{count}/{strategy}: {phase}", flush=True)
                    stream.write(f"\n=== {phase} ===\n")
                    stream.flush()
                    start = time.perf_counter()
                    completed = subprocess.run(
                        cap
                        + [
                            "/usr/bin/time",
                            "-v",
                            "cargo",
                            "build",
                            "--release",
                            "--offline",
                            "--locked",
                            "--manifest-path",
                            str(crate / "Cargo.toml"),
                        ],
                        env=env,
                        stdout=stream,
                        stderr=stream,
                        check=False,
                    )
                    result["builds"].append(
                        {
                            "phase": phase,
                            "seconds": time.perf_counter() - start,
                            "exit": completed.returncode,
                        }
                    )
                    (options.output / "results.json").write_text(
                        json.dumps(metadata, indent=2) + "\n"
                    )
                    if completed.returncode:
                        raise SystemExit(
                            f"capped build failed; retained {log}; no uncapped retry"
                        )
                    if repeat == 0:
                        run(
                            cap
                            + [
                                str(build_dir / "release/dispatch-consumer"),
                                str(crate / "record.xml"),
                            ],
                            env=env,
                            stdout=stream,
                            stderr=stream,
                        )
                        result["roundtrip"] = "passed"
                binary = build_dir / "release/dispatch-consumer"
                result["size_A"] = run(
                    ["size", "-A", str(binary)], capture_output=True
                ).stdout
                result["size"] = run(["size", str(binary)], capture_output=True).stdout
                result["binary_bytes"] = binary.stat().st_size
                result["roundtrip"] = "passed"
                (options.output / "results.json").write_text(
                    json.dumps(metadata, indent=2) + "\n"
                )
                print(result["size_A"], flush=True)


if __name__ == "__main__":
    main()
