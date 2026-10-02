#!/usr/bin/env python3
"""Pinned official UBL 2.4 C# corpus gate (Python stdlib, cargo, .NET 8)."""

import argparse
import hashlib
import json
import subprocess
import tempfile
import urllib.request
import xml.etree.ElementTree as ET
from pathlib import Path
from urllib.parse import urljoin

REPO = Path(__file__).resolve().parent.parent
BASE = "https://docs.oasis-open.org/ubl/os-UBL-2.4/xsd/"
ENTRY = "maindoc/UBL-Invoice-2.4.xsd"


def run(args, cwd=REPO):
    result = subprocess.run(
        [str(a) for a in args], cwd=cwd, capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--schema-dir", type=Path, default=REPO / "target/ubl-2.4/xsd")
    parser.add_argument("--binary", type=Path)
    options = parser.parse_args()
    schema_dir = options.schema_dir.resolve()
    manifest = json.loads((REPO / "research/fixtures/ubl-2.4/sha256.json").read_text())
    for relative, digest in manifest.items():
        path = schema_dir / relative
        if not path.exists():
            data = urllib.request.urlopen(BASE + relative, timeout=60).read()
            if hashlib.sha256(data).hexdigest() != digest:
                raise RuntimeError(f"Official schema hash changed: {relative}")
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"Schema hash mismatch: {relative}")
        for child in ET.parse(path).getroot():
            if child.tag in {
                "{http://www.w3.org/2001/XMLSchema}import",
                "{http://www.w3.org/2001/XMLSchema}include",
            } and child.get("schemaLocation"):
                dependency = urljoin(BASE + relative, child.get("schemaLocation"))
                if (
                    not dependency.startswith(BASE)
                    or dependency[len(BASE) :] not in manifest
                ):
                    raise RuntimeError(f"Unpinned dependency: {dependency}")
    binary = (
        options.binary.resolve() if options.binary else REPO / "target/debug/polyxml"
    )
    if not options.binary:
        run(["cargo", "build", "-p", "polyxml-cli"])
    for scoped in (False, True):
        for style in ("record", "class"):
            with tempfile.TemporaryDirectory(prefix="polyxml-ubl-") as temporary:
                output = Path(temporary)
                args = [
                    binary,
                    "generate",
                    schema_dir / ENTRY,
                    "--lang",
                    "csharp",
                    "--style",
                    style,
                    "--out",
                    output,
                ]
                if scoped:
                    args += ["--root-element", "Invoice"]
                run(args)
                (output / "Corpus.csproj").write_text(
                    '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><Nullable>enable</Nullable><ImplicitUsings>enable</ImplicitUsings></PropertyGroup></Project>'
                )
                (output / "invoice.xml").write_bytes(
                    (REPO / "research/fixtures/ubl-2.4/invoice.xml").read_bytes()
                )
                (output / "Program.cs").write_text(r"""using System.Xml;
using System.Xml.Schema;
using System.Xml.Serialization;
using System.Xml.Linq;
var settings = new XmlReaderSettings { ValidationType = ValidationType.Schema };
settings.Schemas.XmlResolver = new XmlUrlResolver();
settings.Schemas.Add(null, args[0]);
settings.ValidationEventHandler += (_, e) => throw new Exception(e.Message);
void Validate(string xml) { using var reader=XmlReader.Create(new StringReader(xml),settings);while(reader.Read()){} }
var input = File.ReadAllText("invoice.xml"); Validate(input);
var serializer = new XmlSerializer(typeof(Generated.Invoice));
var invoice = (Generated.Invoice)serializer.Deserialize(new StringReader(input))!;
var writer = new StringWriter(); serializer.Serialize(writer, invoice);
var output = writer.ToString(); Validate(output);
var original = XDocument.Parse(input); var result = XDocument.Parse(output);
if (result.Root!.Name != original.Root!.Name) throw new Exception("root QName changed");
foreach (var element in original.Descendants()) {
    if (element.HasElements) continue;
    var expected=original.Descendants(element.Name).Select(e=>e.Value).ToArray();
    var actual=result.Descendants(element.Name).Select(e=>e.Value).ToArray();
    if (!expected.SequenceEqual(actual)) throw new Exception("payload lost: " + element.Name);
    foreach(var attribute in element.Attributes().Where(a=>!a.IsNamespaceDeclaration))
        if (!result.Descendants(element.Name).Any(e=>(string?)e.Attribute(attribute.Name)==attribute.Value)) throw new Exception("attribute lost");
}
serializer.Deserialize(new StringReader(output));
Console.WriteLine("Invoice round trip and independent XSD validation passed");
""")
                run(["dotnet", "build", "Corpus.csproj", "--nologo"], cwd=output)
                run(
                    [
                        "dotnet",
                        "run",
                        "--no-build",
                        "--project",
                        "Corpus.csproj",
                        "--",
                        schema_dir / ENTRY,
                    ],
                    cwd=output,
                )
                print(
                    f"PASS: {'Invoice root' if scoped else 'full graph'}, {style}, {len(manifest)} pinned XSDs",
                    flush=True,
                )


if __name__ == "__main__":
    main()
