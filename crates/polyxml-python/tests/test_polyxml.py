import io
import pathlib
from collections import UserString
from dataclasses import dataclass, field
from decimal import Decimal
from enum import Enum
from xml.etree.ElementTree import QName

import msgspec
import pytest
from pydantic import BaseModel, Field

try:
    from pyxsdata.models.datatype import XmlDate, XmlDateTime, XmlDuration, XmlTime
except ImportError:

    class XmlDate:
        def __init__(self, val: str):
            self.val = val

        @classmethod
        def from_string(cls, val: str):
            return cls(val)

        def __eq__(self, other):
            return isinstance(other, XmlDate) and self.val == other.val

        def __str__(self):
            return self.val

    class XmlDateTime:
        def __init__(self, val: str):
            self.val = val

        @classmethod
        def from_string(cls, val: str):
            return cls(val)

        def __eq__(self, other):
            return isinstance(other, XmlDateTime) and self.val == other.val

        def __str__(self):
            return self.val

    class XmlTime:
        def __init__(self, val: str):
            self.val = val

        @classmethod
        def from_string(cls, val: str):
            return cls(val)

        def __eq__(self, other):
            return isinstance(other, XmlTime) and self.val == other.val

        def __str__(self):
            return self.val

    class XmlDuration:
        def __init__(self, val: str):
            self.val = val

        def __eq__(self, other):
            return isinstance(other, XmlDuration) and self.val == other.val

        def __str__(self):
            return self.val


import polyxml


def test_version():
    assert isinstance(polyxml.__version__, str)
    assert len(polyxml.__version__) > 0


@dataclass
class SimpleItem:
    id: int = field(metadata={"type": "Attribute"})
    name: str = field(metadata={"type": "Element"})
    price: float = field(metadata={"type": "Element"})
    active: bool = field(metadata={"type": "Element"})


def test_deserialize_dataclass_bytes():
    xml = b'<SimpleItem id="42"><name>Sensor</name><price>19.99</price><active>true</active></SimpleItem>'
    res = polyxml.deserialize(xml, SimpleItem)
    assert res.id == 42
    assert res.name == "Sensor"
    assert res.price == 19.99
    assert res.active is True


def test_deserialize_dataclass_str():
    xml_str = '<SimpleItem id="101"><name>Transceiver</name><price>49.50</price><active>1</active></SimpleItem>'
    res = polyxml.deserialize(xml_str, SimpleItem)
    assert res.id == 101
    assert res.name == "Transceiver"
    assert res.price == 49.50
    assert res.active is True


def test_serialize_dataclass():
    item = SimpleItem(id=77, name="Actuator", price=150.0, active=False)
    xml_bytes = polyxml.serialize(item)
    assert b'id="77"' in xml_bytes
    assert b"<name>Actuator</name>" in xml_bytes
    assert b"<price>150" in xml_bytes
    assert b"<active>false</active>" in xml_bytes


def test_serialize_with_indent():
    item = SimpleItem(id=88, name="Turbine", price=999.9, active=True)
    xml_bytes = polyxml.serialize(item, indent=2)
    assert b"  <name>Turbine</name>" in xml_bytes


@dataclass
class ChildNode:
    tag: str = field(metadata={"type": "Element"})


@dataclass
class ContainerNode:
    title: str = field(metadata={"type": "Element"})
    children: list[ChildNode] = field(
        default_factory=list, metadata={"type": "Element", "name": "child"}
    )


def test_nested_dataclasses():
    xml = """
    <ContainerNode>
        <title>Component Tree</title>
        <child><tag>Wing</tag></child>
        <child><tag>Rudder</tag></child>
    </ContainerNode>
    """
    res = polyxml.deserialize(xml, ContainerNode)
    assert res.title == "Component Tree"
    assert len(res.children) == 2
    assert res.children[0].tag == "Wing"
    assert res.children[1].tag == "Rudder"


class PydanticDevice(BaseModel):
    serial: str = Field(..., json_schema_extra={"type": "Attribute", "name": "sn"})
    model: str = Field(..., json_schema_extra={"type": "Element"})
    power: float = Field(..., json_schema_extra={"type": "Element"})


def test_pydantic_model():
    xml = (
        '<PydanticDevice sn="SN-88231"><model>AeroCore</model><power>120.5</power></PydanticDevice>'
    )
    res = polyxml.deserialize(xml, PydanticDevice)
    assert res.serial == "SN-88231"
    assert res.model == "AeroCore"
    assert res.power == 120.5


def test_invalid_xml_raises_value_error():
    with pytest.raises(ValueError):
        polyxml.deserialize(b"<UnclosedTag><name>Test</UnclosedTag", SimpleItem)


def test_iterparse_dataclass_bytes():
    xml = b"""
    <Catalog>
        <header><timestamp>12345</timestamp></header>
        <items>
            <SimpleItem id="1"><name>Widget A</name><price>10.5</price><active>true</active></SimpleItem>
            <SimpleItem id="2"><name>Widget B</name><price>20.0</price><active>false</active></SimpleItem>
            <SimpleItem id="3"><name>Widget C</name><price>30.25</price><active>1</active></SimpleItem>
        </items>
    </Catalog>
    """
    items = list(polyxml.iterparse(xml, SimpleItem))
    assert len(items) == 3
    assert items[0].id == 1 and items[0].name == "Widget A" and items[0].active is True
    assert items[1].id == 2 and items[1].name == "Widget B" and items[1].active is False
    assert items[2].id == 3 and items[2].name == "Widget C" and items[2].active is True


def test_iterparse_custom_tag_and_str():
    xml = """
    <Warehouse>
        <part id="10"><name>Gear</name><price>5.0</price><active>true</active></part>
        <part id="20"><name>Bolt</name><price>0.5</price><active>false</active></part>
    </Warehouse>
    """
    items = list(polyxml.iterparse(xml, SimpleItem, tag="part"))
    assert len(items) == 2
    assert items[0].id == 10 and items[0].name == "Gear"
    assert items[1].id == 20 and items[1].name == "Bolt"


def test_iterparse_empty_stream():
    xml = b"<EmptyCatalog></EmptyCatalog>"
    items = list(polyxml.iterparse(xml, SimpleItem))
    assert len(items) == 0


@dataclass
class OptionalItem:
    id: int = field(metadata={"type": "Attribute"})
    name: str = field(metadata={"type": "Element"})
    desc: str | None = field(default="default_desc", metadata={"type": "Element"})


def test_dataclass_optional_field_fallback():
    # XML omits <desc>, triggering kwargs fallback and preserving dataclass default value
    xml = b'<OptionalItem id="99"><name>Sensor X</name></OptionalItem>'
    res = polyxml.deserialize(xml, OptionalItem)
    assert res.id == 99
    assert res.name == "Sensor X"
    assert res.desc == "default_desc"


def test_serialize_pydantic():
    dev = PydanticDevice(serial="SN-999", model="AeroVibe", power=250.0)
    xml_bytes = polyxml.serialize(dev)
    assert b'sn="SN-999"' in xml_bytes
    assert b"<model>AeroVibe</model>" in xml_bytes
    assert b"<power>250" in xml_bytes


class Status(Enum):
    ACTIVE = "active"
    INACTIVE = "inactive"


class StatusCode(Enum):
    OK = 200
    NOT_FOUND = 404


class NamedStatus(Enum):
    FIRST = 1
    SECOND = 2


@dataclass
class RichTypesItem:
    rate: Decimal = field(metadata={"type": "Element"})
    status: Status = field(metadata={"type": "Element"})
    code: StatusCode = field(metadata={"type": "Element"})
    named: NamedStatus = field(metadata={"type": "Element"})
    date: XmlDate = field(metadata={"type": "Element"})
    datetime: XmlDateTime = field(metadata={"type": "Element"})
    time: XmlTime = field(metadata={"type": "Element"})
    duration: XmlDuration = field(metadata={"type": "Element"})
    tag: str = field(init=False, metadata={"type": "Element"})


def test_rich_types_deserialization_and_serialization():
    xml = (
        b"<RichTypesItem>"
        b"<rate>123.456789</rate>"
        b"<status>active</status>"
        b"<code>200</code>"
        b"<named>FIRST</named>"
        b"<date>2026-09-10</date>"
        b"<datetime>2026-09-10T14:30:00Z</datetime>"
        b"<time>14:30:00</time>"
        b"<duration>P1DT2H</duration>"
        b"<tag>calculated_val</tag>"
        b"</RichTypesItem>"
    )
    res = polyxml.deserialize(xml, RichTypesItem)
    assert res.rate == Decimal("123.456789")
    assert res.status == Status.ACTIVE
    assert res.code == StatusCode.OK
    assert res.named == NamedStatus.FIRST
    assert res.date == XmlDate.from_string("2026-09-10")
    assert res.datetime == XmlDateTime.from_string("2026-09-10T14:30:00Z")
    assert res.time == XmlTime.from_string("14:30:00")
    assert res.duration == XmlDuration("P1DT2H")
    assert res.tag == "calculated_val"

    # Serialize back
    serialized = polyxml.serialize(res)
    assert b"<rate>123.456789</rate>" in serialized
    assert b"<status>active</status>" in serialized
    assert b"<code>200</code>" in serialized
    assert b"<date>2026-09-10</date>" in serialized
    assert b"<tag>calculated_val</tag>" in serialized


def test_to_bytes_pathlib_and_file_str(tmp_path: pathlib.Path):
    xml_file = tmp_path / "item.xml"
    xml_file.write_bytes(
        b'<SimpleItem id="1"><name>A</name><price>1.0</price><active>1</active></SimpleItem>'
    )

    # pathlib.Path
    res1 = polyxml.deserialize(xml_file, SimpleItem)
    assert res1.id == 1

    # str file path
    res2 = polyxml.deserialize(str(xml_file), SimpleItem)
    assert res2.id == 1

    # str raw xml without '<' as first char (e.g. whitespace before, or plain xml)
    raw_str_not_file = (
        "   <SimpleItem id='2'><name>B</name><price>2.0</price><active>0</active></SimpleItem>"
    )
    res3 = polyxml.deserialize(raw_str_not_file, SimpleItem)
    assert res3.id == 2

    # str raw xml that doesn't start with '<' and is not a file
    # (e.g. invalid string) - should fail during parsing or return encoded
    with pytest.raises(ValueError):
        polyxml.deserialize("non_existent_file.xml", SimpleItem)


def test_to_bytes_streams():
    xml = b'<SimpleItem id="10"><name>StreamItem</name><price>3.5</price><active>true</active></SimpleItem>'
    # BytesIO
    res_bytes = polyxml.deserialize(io.BytesIO(xml), SimpleItem)
    assert res_bytes.id == 10

    # StringIO
    res_str = polyxml.deserialize(io.StringIO(xml.decode("utf-8")), SimpleItem)
    assert res_str.id == 10


def test_to_bytes_invalid_type():
    with pytest.raises(TypeError, match="Unsupported XML source type"):
        polyxml.deserialize(12345, SimpleItem)  # type: ignore[arg-type]


@dataclass
class UnionWrapper:
    child: None | ChildNode = field(default=None, metadata={"type": "Element", "name": "child"})
    count: int | None = field(default=None, metadata={"type": "Element"})


def test_pep604_union_nested_dataclass():
    xml = "<UnionWrapper><child><tag>Payload</tag></child><count>42</count></UnionWrapper>"
    res = polyxml.deserialize(xml, UnionWrapper)
    assert res.child is not None
    assert res.child.tag == "Payload"
    assert res.count == 42


def test_lexical_union_tries_scalar_members_in_order():
    from enum import StrEnum
    from typing import Annotated

    class Code(StrEnum):
        LATE = "LATE"
        EARLY = "EARLY"

    type PatternCode = Annotated[str, ("polyxml_patterns", ("[A-Z]{3}[0-9]{2}",))]

    @dataclass
    class Payload:
        number: int | PatternCode = field(metadata={"type": "Element"})
        code: Code | PatternCode = field(metadata={"type": "Element"})

    Payload.__annotations__["number"] = int | PatternCode
    Payload.__annotations__["code"] = Code | PatternCode

    integer = polyxml.deserialize(
        "<Payload><number>42</number><code>LATE</code></Payload>", Payload
    )
    assert integer.number == 42
    assert type(integer.number) is int
    assert integer.code is Code.LATE

    patterned = polyxml.deserialize(
        "<Payload><number>ABC12</number><code>XYZ34</code></Payload>", Payload
    )
    assert patterned.number == "ABC12"
    assert patterned.code == "XYZ34"
    assert type(patterned.code) is str

    with pytest.raises(ValueError, match="union member"):
        polyxml.deserialize("<Payload><number>bad</number><code>LATE</code></Payload>", Payload)


def test_binary_serialization_dataclass():
    item = SimpleItem(id=42, name="TestItem", price=19.99, active=True)
    payload = polyxml.dumps_binary(item)
    assert isinstance(payload, bytes)
    assert len(payload) > 0

    decoded = polyxml.loads_binary(payload, SimpleItem)
    assert isinstance(decoded, SimpleItem)
    assert decoded.id == 42
    assert decoded.name == "TestItem"
    assert decoded.price == 19.99
    assert decoded.active is True


def test_binary_serialization_decimal():
    @dataclass
    class DecimalItem:
        val: Decimal

    item = DecimalItem(val=Decimal("19.99"))
    payload = polyxml.dumps_binary(item)
    decoded = polyxml.loads_binary(payload, DecimalItem)
    assert decoded.val == Decimal("19.99")


def test_binary_serialization_untyped():
    data = {"name": "Alice", "score": 100}
    payload = polyxml.dumps_binary(data)
    decoded = polyxml.loads_binary(payload)
    assert decoded == data


def test_binary_serialization_xmldate_and_path():
    @dataclass
    class ExtraItem:
        date: XmlDate
        path: pathlib.Path

    orig = ExtraItem(date=XmlDate.from_string("2026-09-12"), path=pathlib.Path("/tmp/test.xml"))
    payload = polyxml.dumps_binary(orig)
    decoded = polyxml.loads_binary(payload, ExtraItem)
    assert decoded.date == XmlDate.from_string("2026-09-12")
    assert decoded.path == pathlib.Path("/tmp/test.xml")


def test_binary_serialization_unsupported_type():
    class CustomObject:
        pass

    with pytest.raises(NotImplementedError, match="PolyXML binary serializer cannot serialize"):
        polyxml.dumps_binary(CustomObject())


def test_binary_deserialization_unsupported_type():
    class UnhandledClass:
        pass

    payload = polyxml.dumps_binary({"a": 1})
    with pytest.raises(NotImplementedError, match="PolyXML binary deserializer cannot deserialize"):
        polyxml.loads_binary(payload, UnhandledClass)


class StatusEnum(Enum):
    ACTIVE = "active"
    INACTIVE = "inactive"


class CustomDuration(UserString):
    pass


class FromStringHolder:
    def __init__(self, val: str):
        self.val = val

    @classmethod
    def from_string(cls, val: str):
        return cls(val)

    def __str__(self):
        return self.val

    def __eq__(self, other):
        return isinstance(other, FromStringHolder) and self.val == other.val


@dataclass
class RichXmlItem:
    status: StatusEnum
    qname: QName
    duration: CustomDuration
    holder: FromStringHolder


def test_binary_serialization_xml_types():
    orig = RichXmlItem(
        status=StatusEnum.ACTIVE,
        qname=QName("http://example.com", "elem"),
        duration=CustomDuration("P1Y2M3D"),
        holder=FromStringHolder("parsed_value"),
    )
    payload = polyxml.dumps_binary(orig)
    decoded = polyxml.loads_binary(payload, RichXmlItem)
    assert decoded.status == StatusEnum.ACTIVE
    assert decoded.qname == QName("http://example.com", "elem")
    assert decoded.duration == CustomDuration("P1Y2M3D")
    assert decoded.holder == FromStringHolder("parsed_value")


def test_binary_serialization_tag_class_dataclass():
    item = SimpleItem(id=99, name="TaggedItem", price=49.99, active=True)
    payload = polyxml.dumps_binary(item, tag_class=True)

    # Loads without passing target_type
    decoded_auto = polyxml.loads_binary(payload)
    assert isinstance(decoded_auto, SimpleItem)
    assert decoded_auto.id == 99
    assert decoded_auto.name == "TaggedItem"

    # Loads with explicit target_type on tagged payload
    decoded_explicit = polyxml.loads_binary(payload, SimpleItem)
    assert isinstance(decoded_explicit, SimpleItem)
    assert decoded_explicit.id == 99


def test_binary_serialization_tag_class_pydantic():
    device = PydanticDevice(serial="SN-9999", model="AeroEdge", power=250.0)
    payload = polyxml.dumps_binary(device, tag_class=True)
    decoded = polyxml.loads_binary(payload)
    assert isinstance(decoded, PydanticDevice)
    assert decoded.serial == "SN-9999"
    assert decoded.model == "AeroEdge"
    assert decoded.power == 250.0


def test_binary_serialization_tag_class_primitive():
    payload = polyxml.dumps_binary("plain_string", tag_class=True)
    decoded = polyxml.loads_binary(payload)
    assert decoded == "plain_string"


def test_binary_serialization_hooks_direct():
    from polyxml import _dec_hook, _enc_hook

    assert _enc_hook(StatusEnum.ACTIVE) == "active"
    assert _dec_hook(Decimal, "19.99") == Decimal("19.99")
    assert _dec_hook(StatusEnum, "active") == StatusEnum.ACTIVE


def test_binary_serialization_resolve_class():
    from polyxml import _resolve_class

    assert _resolve_class("") is None
    assert _resolve_class("NoColonHere") is None
    assert _resolve_class("nonexistent.module:FakeClass") is None
    assert _resolve_class("polyxml:dumps_binary") is polyxml.dumps_binary
    # Hits the _CLASS_CACHE branch
    assert _resolve_class("polyxml:dumps_binary") is polyxml.dumps_binary


def test_binary_serialization_tagged_unresolvable_class():
    # Tagged payload with a class that cannot be resolved
    raw_inner = msgspec.msgpack.encode({"key": "value"})
    tagged_data = msgspec.msgpack.encode(("nonexistent.module:FakeClass", raw_inner))

    decoded = polyxml.loads_binary(tagged_data)
    assert decoded == {"key": "value"}


def test_binary_serialization_tagged_payload_decode_error():
    # Explicit target_type on tagged payload where inner payload fails to decode
    raw_inner = msgspec.msgpack.encode({"unexpected": "structure"})
    tagged_data = msgspec.msgpack.encode(("nonexistent:Fake", raw_inner))

    with pytest.raises(msgspec.ValidationError):
        polyxml.loads_binary(tagged_data, SimpleItem)


def test_binary_serialization_target_type_non_tagged_decode_errors():
    # Target type passed with invalid msgpack bytes
    with pytest.raises(msgspec.DecodeError):
        polyxml.loads_binary(b"\xc1", SimpleItem)

    # Target type passed with valid non-tagged msgpack that fails validation
    with pytest.raises(msgspec.ValidationError):
        polyxml.loads_binary(msgspec.msgpack.encode(123), SimpleItem)


def test_binary_serialization_loads_corrupted_or_non_tagged():
    # Plain non-tagged list of two items where second is not bytes
    data = msgspec.msgpack.encode(["not_a_class_tag", 12345])
    decoded = polyxml.loads_binary(data)
    assert decoded == ["not_a_class_tag", 12345]

    # Non-decodable data to custom type
    with pytest.raises(msgspec.DecodeError):
        polyxml.loads_binary(b"\xc1")


def test_mixed_content_wildcard_dataclass():
    @dataclass
    class MixedNode:
        lang: str | None = field(default=None, metadata={"type": "Attribute"})
        content: list[object] = field(
            default_factory=list,
            metadata={"type": "Wildcard", "mixed": True},
        )

    xml = '<MixedNode lang="en">Sample Text Content</MixedNode>'
    decoded = polyxml.deserialize(xml.encode("utf-8"), MixedNode)
    assert decoded.lang == "en"
    assert decoded.content == ["Sample Text Content"]

    serialized = polyxml.serialize(decoded)
    assert b"Sample Text Content" in serialized
    assert b'lang="en"' in serialized


def test_namespaced_dataclass_serialization():
    @dataclass
    class Item:
        class Meta:
            name = "item"
            namespace = "http://example.com/ns1"

        title: str = field(metadata={"type": "Element", "namespace": "http://example.com/ns1"})
        sku: str = field(metadata={"type": "Attribute", "namespace": "http://example.com/ns2"})
        local_attr: int = field(default=99, metadata={"type": "Attribute"})

    item = Item(title="Smartphone", sku="SKU-888", local_attr=99)

    # 1. Auto-detected namespaces
    xml_bytes = polyxml.serialize(item)
    xml_str = xml_bytes.decode("utf-8")
    assert 'xmlns:ns0="http://example.com/ns1"' in xml_str
    assert 'xmlns:ns1="http://example.com/ns2"' in xml_str
    assert xml_str.startswith("<ns0:item")
    assert 'ns1:sku="SKU-888"' in xml_str
    assert 'local_attr="99"' in xml_str
    assert "<ns0:title>Smartphone</ns0:title>" in xml_str
    assert xml_str.endswith("</ns0:item>")

    # 2. Custom ns_map with default namespace
    ns_map = {None: "http://example.com/ns1", "inv": "http://example.com/ns2"}
    xml_custom = polyxml.serialize(item, ns_map=ns_map).decode("utf-8")
    assert 'xmlns="http://example.com/ns1"' in xml_custom
    assert 'xmlns:inv="http://example.com/ns2"' in xml_custom
    assert xml_custom.startswith("<item")
    assert 'inv:sku="SKU-888"' in xml_custom
    assert "<title>Smartphone</title>" in xml_custom
    assert xml_custom.endswith("</item>")

    # 3. Explicitly disabled namespaces (toggle=False)
    xml_raw = polyxml.serialize(item, namespaces=False).decode("utf-8")
    assert "xmlns" not in xml_raw
    assert xml_raw.startswith("<item")
    assert 'sku="SKU-888"' in xml_raw
    assert "<title>Smartphone</title>" in xml_raw


def test_namespaced_pydantic_serialization():
    class PydanticItem(BaseModel):
        class Meta:
            name = "product"
            namespace = "http://example.com/prod"

        name: str = Field(
            json_schema_extra={"type": "Element", "namespace": "http://example.com/prod"}
        )
        code: str = Field(
            json_schema_extra={"type": "Attribute", "namespace": "http://example.com/meta"}
        )

    prod = PydanticItem(name="Tablet", code="TAB-1")
    xml_bytes = polyxml.serialize(prod, namespaces=True)
    xml_str = xml_bytes.decode("utf-8")

    assert 'xmlns:ns0="http://example.com/prod"' in xml_str
    assert 'xmlns:ns1="http://example.com/meta"' in xml_str
    assert xml_str.startswith("<ns0:product")
    assert 'ns1:code="TAB-1"' in xml_str
    assert "<ns0:name>Tablet</ns0:name>" in xml_str
