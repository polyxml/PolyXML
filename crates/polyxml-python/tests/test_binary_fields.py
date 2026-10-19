"""Native codecs preserve Python bytes through cached serde."""

import io
from dataclasses import dataclass, field

import pytest
from pydantic import BaseModel, Field

import polyxml


@pytest.mark.parametrize(("codec", "encoded"), [("base16", "0001FEFF"), ("base64", "AAH+/w==")])
@pytest.mark.parametrize("kind", ["Element", "Attribute", "Text"])
@pytest.mark.parametrize("pydantic", [False, True])
def test_binary_roundtrip(codec, encoded, kind, pydantic):
    metadata = {"type": kind, "format": codec}
    if pydantic:

        class Binary(BaseModel):
            value: bytes = Field(json_schema_extra=metadata)
    else:

        @dataclass
        class Binary:
            value: bytes = field(metadata=metadata)

    value = Binary(value=b"\x00\x01\xfe\xff")
    for _ in range(2):
        xml = polyxml.serialize(value)
        assert encoded.encode() in xml
        parsed = polyxml.deserialize(xml, Binary)
        assert parsed.value == value.value
        assert type(parsed.value) is bytes


@pytest.mark.parametrize("codec", ["base16", "base64"])
@pytest.mark.parametrize("pydantic", [False, True])
def test_binary_optional_repeated_nested_and_empty(codec, pydantic):
    if pydantic:

        class Binary(BaseModel):
            values: list[bytes] = Field(json_schema_extra={"format": codec})
            optional: bytes | None = Field(default=None, json_schema_extra={"format": codec})
    else:

        @dataclass
        class Binary:
            values: list[bytes] = field(metadata={"format": codec})
            optional: bytes | None = field(default=None, metadata={"format": codec})

    @dataclass
    class Container:
        binary: Binary

    for optional in (None, b"", b"\x00\xff"):
        value = Container(Binary(values=[b"", b"\x00\xff", b"123"], optional=optional))
        assert polyxml.deserialize(polyxml.serialize(value), Container) == value


@pytest.mark.parametrize(
    ("codec", "text", "expected"),
    [
        ("base16", " 00 aB\nCD\tff ", b"\x00\xab\xcd\xff"),
        ("base64", " A A\nH+\t/w== ", b"\x00\x01\xfe\xff"),
        ("base16", "", b""),
        ("base64", "", b""),
    ],
)
def test_binary_lexical_input(codec, text, expected):
    @dataclass
    class Binary:
        value: bytes = field(metadata={"format": codec})

    parsed = polyxml.deserialize(f"<Binary><value>{text}</value></Binary>", Binary)
    assert parsed.value == expected
    assert type(parsed.value) is bytes


@pytest.mark.parametrize(
    ("codec", "text"),
    [("base16", "ABC"), ("base16", "GG"), ("base64", "!!!!"), ("base64", "Zg=")],
)
def test_binary_malformed_data(codec, text):
    @dataclass
    class Binary:
        value: bytes = field(metadata={"format": codec})

    with pytest.raises(ValueError, match=f"Invalid {codec} bytes"):
        polyxml.deserialize(f"<Binary><value>{text}</value></Binary>", Binary)


def test_binary_unknown_codec():
    @dataclass
    class Binary:
        value: bytes = field(metadata={"format": "base85"})

    with pytest.raises(ValueError, match="Unsupported bytes format: base85"):
        polyxml.serialize(Binary(b"abc"))


def test_binary_wrong_python_type():
    @dataclass
    class Binary:
        value: bytes = field(metadata={"format": "base16"})

    with pytest.raises(TypeError):
        polyxml.serialize(Binary("ABCD"))


def test_binary_streaming_and_json():
    @dataclass
    class Binary:
        value: bytes = field(metadata={"format": "base16"})

    source = b"<root><Binary><value>00FF</value></Binary></root>"
    assert list(polyxml.iterparse(io.BytesIO(source), Binary, tag="Binary")) == [
        Binary(b"\x00\xff")
    ]
    encoded = polyxml.serialize_json(Binary(b"\x00\xff"))
    assert polyxml.deserialize_json(encoded, Binary) == Binary(b"\x00\xff")


def test_nonbinary_format_metadata_is_unchanged():
    @dataclass
    class Text:
        value: str = field(metadata={"format": "base16"})

    value = Text("00FF")
    assert polyxml.deserialize(polyxml.serialize(value), Text) == value


@pytest.mark.parametrize("codec", ["base16", "base64"])
def test_binary_all_byte_values(codec):
    @dataclass
    class Packet:
        value: bytes = field(metadata={"format": codec})

    for length in range(260):
        value = Packet((bytes(range(256)) * 2)[:length])
        assert polyxml.deserialize(polyxml.serialize(value), Packet) == value


def test_binary_nil_is_distinct_from_empty():
    @dataclass
    class Packet:
        value: bytes | None = field(default=None, metadata={"format": "base16"})

    xml = '<Packet xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><value xsi:nil="true"/></Packet>'
    assert polyxml.deserialize(xml, Packet).value is None
    assert polyxml.deserialize("<Packet><value/></Packet>", Packet).value == b""


@pytest.mark.parametrize("codec", ["base16", "base64"])
@pytest.mark.parametrize("pydantic", [False, True])
def test_binary_empty_text(codec, pydantic):
    metadata = {"type": "Text", "format": codec}
    if pydantic:

        class EmptyText(BaseModel):
            value: bytes = Field(json_schema_extra=metadata)
    else:

        @dataclass
        class EmptyText:
            value: bytes = field(metadata=metadata)

    value = EmptyText(value=b"")
    assert polyxml.deserialize(polyxml.serialize(value), EmptyText) == value
