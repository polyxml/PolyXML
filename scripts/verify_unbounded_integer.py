#!/usr/bin/env python3
"""Audit #117: XML round trips where codecs exist, exact values in every target.

Run with `uv run --with lxml python scripts/verify_unbounded_integer.py` after
building the CLI and reinstalling the Python extension. Requires the seven
language toolchains. TypeScript dependencies are cached under target/.
"""

from pathlib import Path
import json
import subprocess
import tempfile

from lxml import etree

REPO = Path(__file__).resolve().parents[1]
BIN = REPO / "target/debug/polyxml"
SCHEMA = REPO / "research/fixtures/wave5/unbounded_integer.xsd"
DIGITS = [
    "1234567890123456789012345678901234567890",
    "9223372036854775807",
    "9223372036854775808",
    "-9223372036854775808",
    "-9223372036854775809",
    "-1234567890123456789012345678901234567890",
]


def run(args, cwd):
    result = subprocess.run(list(map(str, args)), cwd=cwd, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(f"{' '.join(map(str, args))}\n{result.stdout}\n{result.stderr}")
    return result.stdout


def generate(folder, lang, *options):
    folder.mkdir(parents=True, exist_ok=True)
    run([BIN, "generate", SCHEMA, "--lang", lang, "--out", folder, *options], REPO)


def validate(folder, label):
    schema = etree.XMLSchema(etree.parse(str(SCHEMA)))
    for i, digits in enumerate(DIGITS):
        doc = etree.parse(str(folder / f"{i}.xml"))
        schema.assertValid(doc)
        assert doc.getroot().findtext("Value") == digits, label
    print(f"PASS {label}: independently XSD-valid XML, exact decimal digits", flush=True)


def main():
    with tempfile.TemporaryDirectory(prefix="polyxml-integer-") as temporary:
        root = Path(temporary)
        (root / "digits.json").write_text(json.dumps(DIGITS))
        for i, digits in enumerate(DIGITS):
            (root / f"input{i}.xml").write_text(f"<Root><Value>{digits}</Value></Root>")
        go = root / "go"
        generate(go, "go", "--package", "main")
        (go / "go.mod").write_text("module integers\n\ngo 1.22\n")
        (go / "main.go").write_text("""package main
import("encoding/xml";"encoding/json";"os";"fmt")
func main(){ data,_:=os.ReadFile("../digits.json");var digits []string;json.Unmarshal(data,&digits);for i:=range digits{input,_:=os.ReadFile(fmt.Sprintf("../input%d.xml",i));var root Root;if err:=xml.Unmarshal(input,&root);err!=nil{panic(err)};output,err:=xml.Marshal(root);if err!=nil{panic(err)};os.WriteFile(fmt.Sprintf("%d.xml",i),output,0600)}}
""")
        run(["go", "run", "."], go)
        validate(go, "Go")
        for style in ["record", "class"]:
            cs = root / f"csharp-{style}"
            generate(cs, "csharp", "--style", style)
            (cs / "App.csproj").write_text(
                '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>'
            )
            (cs / "Program.cs").write_text("""using System.Xml.Serialization;using Generated;
var serializer=new XmlSerializer(typeof(Root));for(int i=0;i<6;i++){using var input=File.OpenRead($"../input{i}.xml");var value=serializer.Deserialize(input);using var output=File.Create($"{i}.xml");serializer.Serialize(output,value);}
""")
            run(["dotnet", "run"], cs)
            validate(cs, f"C# {style}")
        for zero_copy in ["true", "false"]:
            rust = root / f"rust-{zero_copy}"
            generate(rust / "src", "rust", "--zero-copy", zero_copy)
            (rust / "Cargo.toml").write_text(f"""[package]
name="integer-audit-{zero_copy}"
version="0.0.0"
edition="2021"
[dependencies]
polyxml={{path={json.dumps(str(REPO / "crates/polyxml-core"))}}}
quick-xml="0.40"
serde={{version="1",features=["derive"]}}
serde_json="1"
""")
            # Match the repository's quick-xml version rather than fetching a different API.
            cargo = (REPO / "crates/polyxml-core/Cargo.toml").read_text()
            import re

            version = re.search(r'quick-xml\s*=\s*\{\s*version\s*=\s*"([^"]+)"', cargo)
            if not version:
                version = re.search(r'quick-xml\s*=\s*"([^"]+)"', cargo)
            manifest = (
                (rust / "Cargo.toml")
                .read_text()
                .replace('quick-xml="0.40"', f'quick-xml="{version.group(1)}"')
            )
            (rust / "Cargo.toml").write_text(manifest)
            (rust / "src/main.rs").write_text("""mod unbounded_integer;use unbounded_integer::Root;
fn main(){for i in 0..6{let input=std::fs::read_to_string(format!("../input{i}.xml")).unwrap();let value=Root::from_xml(&input).unwrap();let mut writer=quick_xml::Writer::new(Vec::new());value.encode_xml(&mut writer,Some("Root")).unwrap();std::fs::write(format!("{i}.xml"),writer.into_inner()).unwrap();}}
""")
            run(
                ["cargo", "run", "--quiet", "--target-dir", REPO / "target/integer-audit-rust"],
                rust,
            )
            validate(rust, f"Rust zero-copy={zero_copy}")
        java = root / "java"
        generate(java, "java", "--style", "pojo", "--feature", "direct-codec", "--package", "audit")
        (
            java / "Main.java"
        ).write_text("""package audit;import java.nio.file.*;import javax.xml.stream.*;
public class Main {public static void main(String[] args)throws Exception{for(int i=0;i<6;i++){var input=XMLInputFactory.newFactory().createXMLStreamReader(Files.newInputStream(Path.of("../input"+i+".xml")));var value=RootTypeCodec.readXml(input);var output=XMLOutputFactory.newFactory().createXMLStreamWriter(Files.newOutputStream(Path.of(i+".xml")));RootTypeCodec.writeXml(value,output);output.close();}}}
""")
        run(["javac", "-d", java, *java.glob("*.java")], java)
        run(["java", "-cp", java, "audit.Main"], java)
        validate(java, "Java BigInteger")
        cpp = root / "cpp"
        generate(cpp, "cpp")
        (cpp / "main.cpp").write_text(
            '#include "unbounded_integer.hpp"\n#include <cassert>\nint main(){polyxml::generated::Root root;root.value="1234567890123456789012345678901234567890";assert(root.validate());root.value="1.0";assert(!root.validate());}\n'
        )
        run(["g++", "-std=c++20", "main.cpp", "-o", "check"], cpp)
        run([cpp / "check"], cpp)
        print("PASS C++: exact string storage and lexical validation (no XML codec)", flush=True)
        cache = REPO / "target/integer-audit-ts"
        cache.mkdir(parents=True, exist_ok=True)
        if not (cache / "node_modules/typescript/bin/tsc").exists():
            (cache / "package.json").write_text('{"private":true,"type":"module"}')
            run(["npm", "install", "typescript", "zod", "valibot", "@sinclair/typebox"], cache)
        for backend in ["zod", "valibot", "typebox"]:
            ts = root / f"ts-{backend}"
            generate(ts, "ts", "--backend", backend)
            (ts / "node_modules").symlink_to(cache / "node_modules", target_is_directory=True)
            (ts / "package.json").write_text('{"type":"module"}')
            run(
                [
                    cache / "node_modules/.bin/tsc",
                    "--skipLibCheck",
                    "--target",
                    "ES2020",
                    "--module",
                    "NodeNext",
                    "--moduleResolution",
                    "NodeNext",
                    "unbounded_integer.ts",
                ],
                ts,
            )
            validation = {
                "zod": 'RootSchema.parse({value:digits});if(RootSchema.safeParse({value:"1.0"}).success)throw Error("invalid accepted")',
                "valibot": 'v.parse(RootSchema,{value:digits});if(v.safeParse(RootSchema,{value:"1.0"}).success)throw Error("invalid accepted")',
                "typebox": 'const validator=TypeCompiler.Compile(RootSchema);if(!validator.Check({value:digits})||validator.Check({value:"1.0"}))throw Error("validation failed")',
            }[backend]
            imports = {
                "zod": "",
                "valibot": 'import * as v from "valibot";',
                "typebox": 'import {TypeCompiler} from "@sinclair/typebox/compiler";',
            }[backend]
            (ts / "run.mjs").write_text(
                f'import {{RootSchema}} from "./unbounded_integer.js";{imports}\nconst digits={json.dumps(DIGITS[0])};{validation};'
            )
            run(["node", "run.mjs"], ts)
            print(
                f"PASS TypeScript {backend}: exact string storage and lexical validation (no XML codec)",
                flush=True,
            )
        for backend in ["dataclass", "pydantic"]:
            py = root / f"python-{backend}"
            generate(py, "python", "--backend", backend)
            (py / "run.py").write_text("""from pathlib import Path
from unbounded_integer import Root
for i in range(6):
    root=Root.from_xml(Path(f"../input{i}.xml").read_text())
    assert type(root.value) is int
    Path(f"{i}.xml").write_bytes(root.to_xml())
""")
            run([REPO / "crates/polyxml-python/.venv/bin/python", "run.py"], py)
            validate(py, f"Python {backend}")


if __name__ == "__main__":
    main()
