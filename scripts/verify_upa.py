#!/usr/bin/env python3
"""Compare XSD 1.0 UPA decisions with Xerces, libxml2, and .NET 8, including seeded content models.

Run `uv run --with lxml python scripts/verify_upa.py` after building the CLI.
"""

import json
import random
import subprocess
import tempfile
from pathlib import Path

from lxml import etree

REPO = Path(__file__).resolve().parents[1]
BIN = REPO / "target/debug/polyxml"
RANDOM = random.Random(131)
RANGES = [
    "",
    ' minOccurs="0"',
    ' minOccurs="0" maxOccurs="0"',
    ' minOccurs="2" maxOccurs="2"',
    ' minOccurs="1" maxOccurs="2"',
    ' minOccurs="0" maxOccurs="unbounded"',
]


def wrap(content):
    if content.startswith(("<xs:element", "<xs:any")):
        content = "<xs:sequence>" + content + "</xs:sequence>"
    return (
        '<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="Root"><xs:complexType>'
        + content
        + "</xs:complexType></xs:element></xs:schema>"
    )


def model(depth):
    if depth == 0 or RANDOM.random() < 0.45:
        name = RANDOM.choice("abc")
        return f'<xs:element name="{name}" type="xs:string"{RANDOM.choice(RANGES)}/>'
    kind = RANDOM.choice(["sequence", "choice"])
    children = "".join(model(depth - 1) for _ in range(RANDOM.randint(1, 3)))
    return f"<xs:{kind}{RANDOM.choice(RANGES)}>{children}</xs:{kind}>"


def main():
    cases = [
        (REPO / "research/fixtures/wave8/30_upa_violation.xsd").read_text(),
        wrap(
            '<xs:sequence><xs:element name="a" type="xs:string"/><xs:element name="a" type="xs:string"/></xs:sequence>'
        ),
        wrap(
            '<xs:sequence><xs:element name="a" type="xs:string" minOccurs="2" maxOccurs="2"/><xs:element name="a" type="xs:string"/></xs:sequence>'
        ),
        wrap(
            '<xs:choice><xs:any namespace="##other" processContents="lax"/><xs:element name="a" type="xs:string"/></xs:choice>'
        ),
        wrap(
            '<xs:choice><xs:any processContents="lax"/><xs:element name="a" type="xs:string"/></xs:choice>'
        ),
        wrap(
            '<xs:choice><xs:any namespace="urn:a" processContents="lax"/><xs:any namespace="urn:b" processContents="lax"/></xs:choice>'
        ),
        wrap(
            '<xs:all><xs:element name="a" type="xs:string"/><xs:element name="a" type="xs:string"/></xs:all>'
        ),
    ]
    cases.extend(wrap(model(3)) for _ in range(500))
    accepted = rejected = libxml_differences = dotnet_differences = 0
    with tempfile.TemporaryDirectory(prefix="polyxml-upa-") as temporary:
        folder = Path(temporary)
        for i, case in enumerate(cases):
            (folder / f"{i}.xsd").write_text(case)
        (folder / "App.csproj").write_text(
            '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings></PropertyGroup></Project>'
        )
        (folder / "Program.cs").write_text(
            'using System.Xml.Schema;using System.Text.Json;\nvar results=new List<bool>();for(int i=0;i<CASES;i++){try{var schemas=new XmlSchemaSet();schemas.CompilationSettings.EnableUpaCheck=true;schemas.Add(null,$"{i}.xsd");schemas.Compile();results.Add(true);}catch(XmlSchemaException){results.Add(false);}}\nConsole.WriteLine(JsonSerializer.Serialize(results));\n'.replace(
                "CASES", str(len(cases))
            )
        )
        oracle = subprocess.run(
            ["dotnet", "run", "--verbosity", "quiet"],
            cwd=folder,
            capture_output=True,
            text=True,
            check=True,
        )
        dotnet = json.loads(oracle.stdout.strip().splitlines()[-1])
        (folder / "Oracle.java").write_text(
            'import javax.xml.validation.*;import javax.xml.XMLConstants;import java.io.*;\nclass Oracle{public static void main(String[]args)throws Exception{var result=new StringBuilder("[");for(int i=0;i<Integer.parseInt(args[0]);i++){if(i>0)result.append(",");try{var factory=SchemaFactory.newInstance(XMLConstants.W3C_XML_SCHEMA_NS_URI);factory.setFeature("http://apache.org/xml/features/validation/schema-full-checking",true);factory.newSchema(new File(i+".xsd"));result.append("true");}catch(org.xml.sax.SAXException e){result.append("false");}}System.out.println(result.append("]"));}}\n'
        )
        subprocess.run(
            ["javac", "Oracle.java"], cwd=folder, check=True, capture_output=True
        )
        oracle = subprocess.run(
            ["java", "-cp", str(folder), "Oracle", str(len(cases))],
            cwd=folder,
            check=True,
            capture_output=True,
            text=True,
        )
        xerces = json.loads(oracle.stdout)
        for i, case in enumerate(cases):
            try:
                etree.XMLSchema(etree.fromstring(case.encode()))
                libxml = True
            except etree.XMLSchemaParseError:
                libxml = False
            source = folder / f"{i}.xsd"
            result = subprocess.run(
                [str(BIN), "validate", str(source)],
                capture_output=True,
                text=True,
                check=False,
            )
            actual = result.returncode == 0
            expected = xerces[i]
            if libxml != expected:
                libxml_differences += 1
            if dotnet[i] != expected:
                dotnet_differences += 1
            if actual != expected:
                failure = REPO / "target/upa-mismatch.xsd"
                failure.write_text(case)
                raise AssertionError(
                    f"Case {i}: Xerces accepted={expected}, .NET accepted={dotnet[i]}, libxml2 accepted={libxml}, PolyXML accepted={actual}\n{result.stdout}{result.stderr}\nSaved {failure}"
                )
            if actual:
                accepted += 1
            else:
                rejected += 1
    print(
        f"PASS: {len(cases)} decisions match Xerces ({accepted} accepted, {rejected} rejected); libxml2 differs on {libxml_differences}, .NET 8 differs on {dotnet_differences}"
    )


if __name__ == "__main__":
    main()
