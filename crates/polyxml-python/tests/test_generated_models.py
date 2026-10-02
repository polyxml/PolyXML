import importlib.util
import pathlib
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from decimal import Decimal

import pytest

import polyxml


@pytest.mark.parametrize("backend", ["dataclass", "pydantic"])
def test_generated_abstract_root_preserves_derived_payload(tmp_path, backend):
    schema = (
        pathlib.Path(__file__).resolve().parents[3]
        / "research/fixtures/wave6/abstract_derived_xsi.xsd"
    )
    subprocess.run(
        [
            str(_get_polyxml_bin()),
            "generate",
            str(schema),
            "--lang",
            "python",
            "--backend",
            backend,
            "--out",
            str(tmp_path),
        ],
        check=True,
        capture_output=True,
    )
    module = _load_module_from_file(
        f"abstract_root_{backend}", tmp_path / "abstract_derived_xsi.py"
    )
    xml = '<document xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:type="ConcreteDocument"><info>payload</info></document>'
    model = module.Document.from_xml(xml)
    assert isinstance(model, module.ConcreteDocument)
    assert model.info == "payload"
    encoded = model.to_xml()
    parsed = ET.fromstring(encoded)
    assert parsed.tag == "document"
    assert parsed.findtext("info") == "payload"
    assert parsed.get("{http://www.w3.org/2001/XMLSchema-instance}type") == "ConcreteDocument"
    assert module.Document.from_xml(encoded).info == "payload"
    with pytest.raises(ValueError, match="Unexpected root"):
        module.Document.from_xml(xml.replace("document", "wrong"))


def test_roots_with_the_same_local_name_in_distinct_namespaces(tmp_path):
    base = tmp_path / "base.xsd"
    base.write_text(
        '<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:base" xmlns:b="urn:base"><xs:complexType name="A"><xs:sequence><xs:element name="BaseValue" type="xs:string"/></xs:sequence></xs:complexType><xs:element name="a" type="b:A"/></xs:schema>'
    )
    main = tmp_path / "main.xsd"
    main.write_text(
        '<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:main" xmlns:m="urn:main"><xs:import namespace="urn:base" schemaLocation="base.xsd"/><xs:complexType name="A"><xs:sequence><xs:element name="MainValue" type="xs:string"/></xs:sequence></xs:complexType><xs:element name="a" type="m:A"/></xs:schema>'
    )
    out = tmp_path / "models"
    subprocess.run(
        [str(_get_polyxml_bin()), "generate", str(main), "--lang", "python", "--out", str(out)],
        check=True,
        capture_output=True,
    )
    module = _load_module_from_file("namespace_roots", out / "main.py")
    root = module.AElement2.from_xml('<a xmlns="urn:main"><MainValue>main</MainValue></a>')
    assert root.main_value == "main"
    assert ET.fromstring(root.to_xml()).tag == "{urn:main}a"
    with pytest.raises(ValueError, match="Unexpected root"):
        module.AElement2.from_xml('<a xmlns="urn:base"><BaseValue>base</BaseValue></a>')


@pytest.mark.parametrize("backend", ["dataclass", "pydantic"])
@pytest.mark.parametrize(
    ("fixture", "root_name", "documents"),
    [
        (
            "nested_sequence_choice.xsd",
            "Root",
            [
                "<Root><First>A</First><Second>B</Second></Root>",
                "<Root><Alternative>C</Alternative></Root>",
            ],
        ),
        (
            "repeated_sequence.xsd",
            "Root",
            [
                "<Root><First>A1</First><Second>B1</Second><First>A2</First><Second>B2</Second></Root>",
                "<Root/>",
            ],
        ),
        (
            "substitution_group.xsd",
            "Portfolio",
            [
                "<Portfolio xmlns='urn:audit:substitution'><Bond>A1</Bond><Equity>B1</Equity><Bond>A2</Bond></Portfolio>"
            ],
        ),
        (
            "choice_branch_cardinality.xsd",
            "Root",
            [
                "<Root><Timing>A1</Timing><Timing>A2</Timing></Root>",
                "<Root><Drive>B</Drive></Root>",
            ],
        ),
        (
            "wave6/duplicate_choice_branch_name.xsd",
            "Person",
            [
                "<Person><MinAge>18</MinAge><MaxAge>25</MaxAge></Person>",
                "<Person><MaxAge>25</MaxAge></Person>",
                "<Person/>",
            ],
        ),
    ],
)
def test_particle_codec_regressions(tmp_path, backend, fixture, root_name, documents):
    schema = pathlib.Path(__file__).resolve().parents[3] / "research" / "fixtures" / fixture
    subprocess.run(
        [
            str(_get_polyxml_bin()),
            "generate",
            str(schema),
            "--lang",
            "python",
            "--backend",
            backend,
            "--out",
            str(tmp_path),
        ],
        check=True,
        capture_output=True,
    )
    module = _load_module_from_file(
        f"particles_{backend}_{schema.stem}", tmp_path / f"{schema.stem}.py"
    )
    root_type = getattr(module, root_name)

    def structure(element):
        return (
            element.tag,
            (element.text or "").strip(),
            tuple(structure(child) for child in element),
        )

    for xml in documents:
        decoded = root_type.from_xml(xml)
        encoded = decoded.to_xml()
        assert structure(ET.fromstring(encoded)) == structure(ET.fromstring(xml))
        assert structure(ET.fromstring(root_type.from_xml(encoded).to_xml())) == structure(
            ET.fromstring(xml)
        )


SAMPLE_XSD = """<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="https://example.com/warehouse"
           xmlns="https://example.com/warehouse"
           elementFormDefault="qualified">

    <xs:simpleType name="PartStatus">
        <xs:restriction base="xs:string">
            <xs:enumeration value="in-stock"/>
            <xs:enumeration value="back-ordered"/>
            <xs:enumeration value="discontinued"/>
        </xs:restriction>
    </xs:simpleType>

    <xs:simpleType name="PartSku">
        <xs:restriction base="xs:string">
            <xs:minLength value="4"/>
            <xs:maxLength value="12"/>
            <xs:pattern value="[A-Z]{3}-[0-9]{4}"/>
        </xs:restriction>
    </xs:simpleType>

    <xs:simpleType name="Quantity">
        <xs:restriction base="xs:integer">
            <xs:minInclusive value="0"/>
            <xs:maxInclusive value="9999"/>
        </xs:restriction>
    </xs:simpleType>

    <xs:complexType name="Dimension">
        <xs:sequence>
            <xs:element name="width" type="xs:double"/>
            <xs:element name="height" type="xs:double"/>
            <xs:element name="depth" type="xs:double"/>
        </xs:sequence>
    </xs:complexType>

    <xs:complexType name="PartItem">
        <xs:sequence>
            <xs:element name="sku" type="PartSku"/>
            <xs:element name="name" type="xs:string"/>
            <xs:element name="status" type="PartStatus"/>
            <xs:element name="quantity" type="Quantity"/>
            <xs:element name="price" type="xs:decimal"/>
            <xs:element name="description" type="xs:string" minOccurs="0" nillable="true"/>
            <xs:element name="dimension" type="Dimension" minOccurs="0"/>
            <xs:element name="tags" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
        </xs:sequence>
        <xs:attribute name="id" type="xs:int" use="required"/>
        <xs:attribute name="active" type="xs:boolean" default="true"/>
    </xs:complexType>

    <xs:complexType name="Inventory">
        <xs:sequence>
            <xs:element name="warehouseName" type="xs:string"/>
            <xs:element name="items" type="PartItem" minOccurs="0" maxOccurs="unbounded"/>
        </xs:sequence>
    </xs:complexType>

    <xs:element name="WarehouseInventory" type="Inventory"/>
</xs:schema>
"""

SAMPLE_XML = b"""<Inventory xmlns="https://example.com/warehouse">
    <warehouseName>Central Distribution</warehouseName>
    <items id="101" active="true">
        <sku>ENG-1234</sku>
        <name>Gearbox Module</name>
        <status>in-stock</status>
        <quantity>45</quantity>
        <price>499.95</price>
        <description>Heavy duty aerospace gearbox</description>
        <dimension>
            <width>12.5</width>
            <height>8.0</height>
            <depth>15.2</depth>
        </dimension>
        <tags>aviation</tags>
        <tags>powertrain</tags>
    </items>
    <items id="102" active="false">
        <sku>HYD-5678</sku>
        <name>Hydraulic Valve</name>
        <status>back-ordered</status>
        <quantity>0</quantity>
        <price>89.50</price>
        <description nil="true"/>
    </items>
</Inventory>
"""


def _load_module_from_file(module_name: str, file_path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(module_name, file_path)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def _get_polyxml_bin() -> pathlib.Path:
    repo_root = pathlib.Path(__file__).parent.parent.parent.parent
    exe_name = "polyxml.exe" if sys.platform == "win32" else "polyxml"
    candidates = [
        repo_root / "target" / "debug" / exe_name,
        repo_root / "target" / "release" / exe_name,
    ]
    for c in candidates:
        if c.exists():
            return c

    which_path = shutil.which(exe_name) or shutil.which("polyxml")
    if which_path:
        return pathlib.Path(which_path)

    # Attempt on-the-fly compilation via cargo if not found
    subprocess.run(
        ["cargo", "build", "-p", "polyxml-cli"],
        cwd=repo_root,
        check=True,
        capture_output=True,
    )
    for c in candidates:
        if c.exists():
            return c

    raise FileNotFoundError(f"polyxml CLI binary could not be found or built at {candidates}")


@pytest.fixture(scope="module")
def generated_models():
    """Generates both Dataclass and Pydantic models from SAMPLE_XSD using polyxml CLI,
    verifying ruff and pyright compliance."""
    polyxml_bin = _get_polyxml_bin()

    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = pathlib.Path(tmpdir)
        xsd_file = tmp_path / "warehouse.xsd"
        xsd_file.write_text(SAMPLE_XSD)

        # 1. Generate Dataclass models
        dc_dir = tmp_path / "gen_dc"
        dc_dir.mkdir()
        res_dc = subprocess.run(
            [
                str(polyxml_bin),
                "generate",
                "--lang",
                "python",
                "--backend",
                "dataclass",
                "--out",
                str(dc_dir),
                str(xsd_file),
                "--format",
            ],
            capture_output=True,
            text=True,
        )
        assert res_dc.returncode == 0, f"Dataclass codegen failed: {res_dc.stderr}"
        dc_file = dc_dir / "warehouse.py"
        assert dc_file.exists()

        # 2. Generate Pydantic models
        pyd_dir = tmp_path / "gen_pyd"
        pyd_dir.mkdir()
        res_pyd = subprocess.run(
            [
                str(polyxml_bin),
                "generate",
                "--lang",
                "python",
                "--backend",
                "pydantic",
                "--out",
                str(pyd_dir),
                str(xsd_file),
                "--format",
            ],
            capture_output=True,
            text=True,
        )
        assert res_pyd.returncode == 0, f"Pydantic codegen failed: {res_pyd.stderr}"
        pyd_file = pyd_dir / "warehouse.py"
        assert pyd_file.exists()

        # 3. Verify ruff check on both
        ruff_check_dc = subprocess.run(
            ["ruff", "check", str(dc_dir)], capture_output=True, text=True
        )
        assert ruff_check_dc.returncode == 0, (
            f"Ruff check failed on dataclasses: {ruff_check_dc.stdout}\n{ruff_check_dc.stderr}"
        )

        ruff_check_pyd = subprocess.run(
            ["ruff", "check", str(pyd_dir)], capture_output=True, text=True
        )
        assert ruff_check_pyd.returncode == 0, (
            f"Ruff check failed on pydantic: {ruff_check_pyd.stdout}\n{ruff_check_pyd.stderr}"
        )

        # 4. Verify pyright static type checker on both if installed
        if shutil.which("pyright"):
            pyright_dc = subprocess.run(["pyright", str(dc_file)], capture_output=True, text=True)
            assert pyright_dc.returncode == 0, f"Pyright failed on dataclasses: {pyright_dc.stdout}"

            pyright_pyd = subprocess.run(["pyright", str(pyd_file)], capture_output=True, text=True)
            assert pyright_pyd.returncode == 0, f"Pyright failed on pydantic: {pyright_pyd.stdout}"

        # 5. Load modules dynamically
        mod_dc = _load_module_from_file("gen_warehouse_dc", dc_file)
        mod_pyd = _load_module_from_file("gen_warehouse_pyd", pyd_file)

        yield {"dataclass": mod_dc, "pydantic": mod_pyd}


@pytest.mark.parametrize("backend", ["dataclass", "pydantic"])
def test_declared_root_models(generated_models, backend):
    mod = generated_models[backend]
    xml = SAMPLE_XML.replace(b"Inventory", b"WarehouseInventory")
    root = mod.WarehouseInventory.from_xml(xml)
    assert isinstance(root, mod.WarehouseInventory)
    assert b"WarehouseInventory" in root.to_xml()
    decoded = mod.WarehouseInventory.from_xml(root.to_xml())
    assert decoded.warehouse_name == root.warehouse_name
    with pytest.raises(ValueError, match="Unexpected root"):
        mod.WarehouseInventory.from_xml(SAMPLE_XML)
    with pytest.raises(ValueError, match="Unexpected root"):
        mod.WarehouseInventory.from_xml(xml.replace(b"https://example.com/warehouse", b"urn:wrong"))
    with pytest.raises(ValueError, match="Unexpected root"):
        mod.WarehouseInventory.from_xml(b"<Other/>")


def test_generated_dataclass_deserialization_and_serialization(generated_models):
    mod = generated_models["dataclass"]

    # Verify Enum
    assert hasattr(mod, "PartStatus")
    assert mod.PartStatus.IN_STOCK.value == "in-stock"
    assert mod.PartStatus.BACK_ORDERED.value == "back-ordered"

    # Deserialize
    inv = polyxml.deserialize(SAMPLE_XML, mod.Inventory)
    assert inv.warehouse_name == "Central Distribution"
    assert len(inv.items) == 2

    # Check item 0
    item0 = inv.items[0]
    assert item0.id == 101
    assert item0.active is True
    assert item0.sku == "ENG-1234"
    assert item0.name == "Gearbox Module"
    assert item0.status == mod.PartStatus.IN_STOCK
    assert item0.quantity == 45
    assert item0.price == Decimal("499.95")
    assert item0.description == "Heavy duty aerospace gearbox"
    assert item0.dimension is not None
    assert item0.dimension.width == 12.5
    assert item0.dimension.height == 8.0
    assert item0.dimension.depth == 15.2
    assert item0.tags == ["aviation", "powertrain"]

    # Check item 1
    item1 = inv.items[1]
    assert item1.id == 102
    assert item1.active is False
    assert item1.sku == "HYD-5678"
    assert item1.status == mod.PartStatus.BACK_ORDERED
    assert item1.quantity == 0
    assert item1.price == Decimal("89.50")
    assert item1.dimension is None

    # Reserialize
    xml_out = polyxml.serialize(inv)
    assert b"Central Distribution" in xml_out
    assert b"ENG-1234" in xml_out
    assert b"HYD-5678" in xml_out
    assert b"aviation" in xml_out


def test_generated_pydantic_deserialization_and_serialization(generated_models):
    mod = generated_models["pydantic"]

    assert hasattr(mod, "PartStatus")
    assert hasattr(mod, "Inventory")
    assert hasattr(mod, "PartItem")

    # Deserialize into Pydantic model
    inv = polyxml.deserialize(SAMPLE_XML, mod.Inventory)
    assert inv.warehouse_name == "Central Distribution"
    assert len(inv.items) == 2

    item0 = inv.items[0]
    assert item0.id == 101
    assert item0.sku == "ENG-1234"
    assert item0.price == Decimal("499.95")
    assert item0.quantity == 45
    assert item0.tags == ["aviation", "powertrain"]

    # Test Pydantic model dump
    dumped = inv.model_dump()
    assert dumped["warehouse_name"] == "Central Distribution"
    assert len(dumped["items"]) == 2

    # Reserialize back to XML
    xml_out = polyxml.serialize(inv)
    assert b"Central Distribution" in xml_out
    assert b"ENG-1234" in xml_out

    # Test inherent codecs on Pydantic model: from_xml and to_xml
    inv_codec = mod.Inventory.from_xml(SAMPLE_XML)
    assert inv_codec.warehouse_name == "Central Distribution"
    assert len(inv_codec.items) == 2

    # String input support
    inv_str = mod.Inventory.from_xml(SAMPLE_XML.decode("utf-8"))
    assert inv_str.warehouse_name == "Central Distribution"

    # to_xml support with indentation
    xml_codec_bytes = inv_codec.to_xml(indent=2)
    assert b"Central Distribution" in xml_codec_bytes
    assert b"\n" in xml_codec_bytes


def test_generated_dataclass_codecs(generated_models):
    mod = generated_models["dataclass"]

    # Inherent from_xml on Dataclass model
    inv = mod.Inventory.from_xml(SAMPLE_XML)
    assert inv.warehouse_name == "Central Distribution"
    assert len(inv.items) == 2
    assert inv.items[0].sku == "ENG-1234"

    # String input
    inv_str = mod.Inventory.from_xml(SAMPLE_XML.decode("utf-8"))
    assert inv_str.warehouse_name == "Central Distribution"

    # Inherent to_xml
    xml_bytes = inv.to_xml(indent=4)
    assert b"Central Distribution" in xml_bytes
    assert b"ENG-1234" in xml_bytes


def test_codecs_flag_disabled():
    polyxml_bin = _get_polyxml_bin()

    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = pathlib.Path(tmpdir)
        xsd_file = tmp_path / "warehouse.xsd"
        xsd_file.write_text(SAMPLE_XSD)

        out_dir = tmp_path / "no_codecs"
        out_dir.mkdir()
        res = subprocess.run(
            [
                str(polyxml_bin),
                "generate",
                "--lang",
                "python",
                "--codecs",
                "false",
                "--out",
                str(out_dir),
                str(xsd_file),
            ],
            capture_output=True,
            text=True,
        )
        assert res.returncode == 0
        mod = _load_module_from_file("gen_no_codecs", out_dir / "warehouse.py")
        assert not hasattr(mod.Inventory, "from_xml")
        assert not hasattr(mod.Inventory, "to_xml")


@pytest.mark.parametrize("backend", ["dataclass", "pydantic"])
def test_generated_models_reject_general_entities(tmp_path, backend):
    schema = pathlib.Path(__file__).resolve().parents[3] / "research/fixtures/wave7/xxe_text.xsd"
    subprocess.run(
        [
            str(_get_polyxml_bin()),
            "generate",
            str(schema),
            "--lang",
            "python",
            "--backend",
            backend,
            "--out",
            str(tmp_path),
        ],
        check=True,
        capture_output=True,
    )
    module = _load_module_from_file(f"entities_{backend}", tmp_path / "xxe_text.py")
    sentinel = tmp_path / "sentinel.txt"
    sentinel.write_text("PRIVATE_SENTINEL_VALUE")
    for declaration in [
        "",
        "<!DOCTYPE Document [<!ENTITY audit 'EXPECTED'>]>",
        f"<!DOCTYPE Document [<!ENTITY audit SYSTEM '{sentinel.as_uri()}'>]>",
    ]:
        with pytest.raises(ValueError, match="Unsupported general entity reference"):
            module.Document.from_xml(
                declaration + "<Document><Payload>&audit;</Payload></Document>"
            )
    model = module.Document.from_xml("<Document><Payload>&amp;&#65;&#x42;</Payload></Document>")
    assert model.payload == "&AB"


@pytest.mark.parametrize("backend", ["dataclass", "pydantic"])
def test_element_defaults_preserve_absence(tmp_path, backend):
    schema = pathlib.Path(__file__).resolve().parents[3] / "research/fixtures/element_defaults.xsd"
    subprocess.run(
        [
            str(_get_polyxml_bin()),
            "generate",
            str(schema),
            "--lang",
            "python",
            "--backend",
            backend,
            "--out",
            str(tmp_path),
        ],
        check=True,
        capture_output=True,
    )
    module = _load_module_from_file(f"defaults_{backend}", tmp_path / "element_defaults.py")
    absent = module.Root.from_xml("<Root/>")
    assert (absent.flag, absent.count, absent.label, absent.mode) == (None, None, None, "auto")
    assert ET.fromstring(absent.to_xml()).find("Flag") is None
    for document in [
        "<Root><Flag/><Count></Count><Label/></Root>",
        "<Root><Flag></Flag><Count/><Label></Label></Root>",
    ]:
        present = module.Root.from_xml(document)
        assert (present.flag, present.count, present.label) == (False, 42, "fallback")
        encoded = present.to_xml()
        assert ET.fromstring(encoded).findtext("Flag") == "false"
        assert module.Root.from_xml(encoded).count == 42
