"""PolyXML: High-performance, polyglot native XML data-binding engine.

QUICKSTART FOR AI AGENTS & DEVELOPERS:

1. Deserialize XML (string, bytes, or file Path) into a typed Dataclass or Pydantic model:
    >>> import polyxml
    >>> from dataclasses import dataclass, field
    >>>
    >>> @dataclass
    >>> class User:
    ...     name: str = field(metadata={"type": "Element"})
    >>>
    >>> user = polyxml.deserialize("<User><name>Alice</name></User>", User)
    >>> user.name
    'Alice'

2. Stream large XML files with O(1) constant memory (Streaming / Facet Extraction):
    >>> for user in polyxml.iterparse("massive_users.xml", User, tag="User"):
    ...     process(user)

3. Serialize a model instance back into XML bytes:
    >>> xml_bytes = polyxml.serialize(user, indent=2)

See `AGENT_GUIDE.md` or https://polyxml.github.io/PolyXML/ for complete documentation.
"""

import importlib
import pathlib
from collections import UserString
from collections.abc import Iterator
from dataclasses import dataclass, field, is_dataclass
from decimal import Decimal
from enum import Enum
from typing import IO
from xml.etree.ElementTree import QName

from polyxml._polyxml import (  # type: ignore[import-not-found]
    deserialize as _deserialize,
)
from polyxml._polyxml import (
    deserialize_json as _deserialize_json,
)
from polyxml._polyxml import (
    iterparse as _iterparse,
)
from polyxml._polyxml import (
    json_to_xml as _json_to_xml,
)
from polyxml._polyxml import (
    serialize as _serialize,
)
from polyxml._polyxml import (
    serialize_json as _serialize_json,
)
from polyxml._polyxml import (
    version as _version,
)
from polyxml._polyxml import (
    xml_to_json as _xml_to_json,
)

__version__: str = _version()


def _to_bytes(source: bytes | str | pathlib.Path | IO[bytes] | IO[str]) -> bytes:
    if isinstance(source, bytes):
        return source
    if isinstance(source, str):
        stripped = source.lstrip()
        if stripped.startswith(("<", "{", "[")):
            return source.encode("utf-8")
        path = pathlib.Path(source)
        if path.is_file():
            return path.read_bytes()
        return source.encode("utf-8")
    if isinstance(source, pathlib.Path):
        return source.read_bytes()
    if hasattr(source, "read"):
        data = source.read()
        return data.encode("utf-8") if isinstance(data, str) else data
    raise TypeError(
        f"Unsupported XML source type: {type(source).__name__}. "
        "Expected bytes, str (XML string or file path), pathlib.Path, or a binary/text IO stream."
    )


def deserialize[T](
    source: bytes | str | pathlib.Path | IO[bytes] | IO[str], target_type: type[T]
) -> T:
    """Deserialize XML bytes, string, file path, or stream into a typed model.

    Args:
        source: XML content as raw bytes, string, Path, or file stream.
        target_type: The target dataclass or model class.

    Returns:
        The deserialized model instance.
    """
    return _deserialize(_to_bytes(source), target_type)


def deserialize_json[T](
    source: bytes | str | pathlib.Path | IO[bytes] | IO[str], target_type: type[T]
) -> T:
    """Deserialize JSON bytes, string, file path, or stream into a typed model.

    Args:
        source: JSON content as raw bytes, string, Path, or file stream.
        target_type: The target dataclass or model class.

    Returns:
        The deserialized model instance.
    """
    return _deserialize_json(_to_bytes(source), target_type)


def loads_json[T](
    source: bytes | str | pathlib.Path | IO[bytes] | IO[str], target_type: type[T]
) -> T:
    """Deserialize JSON bytes, string, file path, or stream into a typed model (alias for deserialize_json)."""
    return deserialize_json(source, target_type)


def iterparse[T](
    source: bytes | str | pathlib.Path | IO[bytes] | IO[str],
    target_type: type[T],
    tag: str | None = None,
) -> Iterator[T]:
    """Stream and deserialize XML elements one-by-one with O(1) constant memory.

    Args:
        source: XML content as raw bytes, string, Path, or file stream.
        target_type: The target dataclass or model class for each element.
        tag: Optional XML element tag name to match. Defaults to the model schema name.

    Yields:
        Deserialized model instances as they are streamed.
    """
    return _iterparse(_to_bytes(source), target_type, tag)


def serialize(
    obj: object,
    *,
    target_type: type[object] | None = None,
    indent: int | None = None,
    namespaces: bool | None = None,
    ns_map: dict[str, str] | None = None,
) -> bytes:
    """Serialize a strongly-typed model instance back into XML bytes.

    Args:
        obj: Python dataclass or Pydantic model instance.
        target_type: Optional declared base type for ``xsi:type`` polymorphic
            dispatch. When given, a concrete subclass instance is serialized
            using the base type's element name and re-emits an ``xsi:type``
            selector naming the concrete type, so polymorphic round trips
            preserve the wire form. When None (default), the instance's own
            concrete type is used.
        indent: Optional indentation size in spaces for pretty-printing.
        namespaces: Optional boolean toggle to enable or disable XML namespace prefix
            resolution and root xmlns attribute generation. When None (default),
            namespaces are automatically enabled if the model or fields define
            namespaces or if ns_map is provided.
        ns_map: Optional prefix-to-URI or URI-to-prefix mapping dictionary for XML namespaces.

    Returns:
        UTF-8 encoded XML bytes representing the model instance.
    """
    return _serialize(
        obj,
        target_type=target_type,
        indent=indent,
        namespaces=namespaces,
        ns_map=ns_map,
    )


def serialize_json(
    obj: object,
    *,
    indent: int | None = None,
    by_alias: bool = True,
) -> bytes:
    """Serialize a strongly-typed model instance into JSON bytes.

    Args:
        obj: Python dataclass or Pydantic model instance.
        indent: Optional indentation size in spaces for pretty-printing.
        by_alias: Whether to serialize fields using schema aliases (xml_name) or internal attribute names.

    Returns:
        UTF-8 encoded JSON bytes representing the model instance.
    """
    return _serialize_json(obj, indent=indent, by_alias=by_alias)


def dumps_json(
    obj: object,
    *,
    indent: int | None = None,
    by_alias: bool = True,
) -> str:
    """Serialize a strongly-typed model instance into a JSON string.

    Args:
        obj: Python dataclass or Pydantic model instance.
        indent: Optional indentation size in spaces for pretty-printing.
        by_alias: Whether to serialize fields using schema aliases (xml_name) or internal attribute names.

    Returns:
        JSON string representing the model instance.
    """
    return serialize_json(obj, indent=indent, by_alias=by_alias).decode("utf-8")


def xml_to_json(
    source: bytes | str | pathlib.Path | IO[bytes] | IO[str],
    target_type: type[object] | None = None,
    *,
    model: type[object] | None = None,
    schema_path: str | pathlib.Path | None = None,
    root: str | None = None,
    indent: int | None = None,
    by_alias: bool = True,
) -> bytes:
    """Transcode XML into JSON bytes directly in C/Rust.

    Args:
        source: XML content as raw bytes, string, Path, or file stream.
        target_type: Optional Python dataclass/model type to guide schema-directed transcoding.
        model: Alias for target_type.
        schema_path: Optional path to an XSD schema file.
        root: Optional root element name when using schema_path.
        indent: Optional indentation size in spaces for pretty-printed JSON.
        by_alias: Whether to use XML tag aliases as JSON keys (default True).

    Returns:
        Transcoded UTF-8 JSON bytes.
    """
    effective_type = model if model is not None else target_type
    path_str = str(schema_path) if schema_path is not None else None
    return _xml_to_json(
        _to_bytes(source),
        target_type=effective_type,
        schema_path=path_str,
        root=root,
        indent=indent,
        by_alias=by_alias,
    )


def json_to_xml(
    source: bytes | str | pathlib.Path | IO[bytes] | IO[str],
    target_type: type[object] | None = None,
    *,
    model: type[object] | None = None,
    schema_path: str | pathlib.Path | None = None,
    root: str | None = None,
    indent: int | None = None,
    namespaces: bool | None = None,
    ns_map: dict[str, str] | None = None,
) -> bytes:
    """Transcode JSON into XML bytes directly in C/Rust.

    Args:
        source: JSON content as raw bytes, string, Path, or file stream.
        target_type: Optional Python dataclass/model type to guide schema-directed transcoding.
        model: Alias for target_type.
        schema_path: Optional path to an XSD schema file.
        root: Optional root XML element tag name.
        indent: Optional indentation size in spaces for pretty-printed XML.
        namespaces: Optional boolean toggle for XML namespaces.
        ns_map: Optional prefix-to-URI or URI-to-prefix mapping dictionary.

    Returns:
        Transcoded UTF-8 XML bytes.
    """
    effective_type = model if model is not None else target_type
    path_str = str(schema_path) if schema_path is not None else None
    return _json_to_xml(
        _to_bytes(source),
        target_type=effective_type,
        schema_path=path_str,
        root=root,
        indent=indent,
        namespaces=namespaces,
        ns_map=ns_map,
    )


@dataclass(slots=True)
class AnyElement:
    """Represents an unmapped wildcard XML element (xs:any)."""

    qname: str | None = None
    text: str | None = None
    children: list[object] = field(default_factory=list)
    attributes: dict[str, str] = field(default_factory=dict)


class JsonSerializer:
    """Drop-in xsdata-compatible JSON serializer."""

    def __init__(
        self,
        indent: int | None = None,
        by_alias: bool = True,
        **_kwargs: object,
    ) -> None:
        self.indent = indent
        self.by_alias = by_alias

    def render(self, obj: object) -> str:
        """Render a model instance into a JSON string."""
        return dumps_json(obj, indent=self.indent, by_alias=self.by_alias)


class JsonParser:
    """Drop-in xsdata-compatible JSON parser."""

    def __init__(self, **_kwargs: object) -> None:
        pass

    def from_string[T](self, source: str, clazz: type[T]) -> T:
        """Parse a JSON string into a model instance."""
        return deserialize_json(source, clazz)

    def from_bytes[T](self, source: bytes, clazz: type[T]) -> T:
        """Parse JSON bytes into a model instance."""
        return deserialize_json(source, clazz)

    def parse[T](
        self, source: bytes | str | pathlib.Path | IO[bytes] | IO[str], clazz: type[T]
    ) -> T:
        """Parse a JSON source into a model instance."""
        return deserialize_json(source, clazz)


class XmlSerializer:
    """Drop-in xsdata-compatible XML serializer."""

    def __init__(
        self,
        indent: int | None = None,
        namespaces: bool | None = None,
        ns_map: dict[str, str] | None = None,
        **_kwargs: object,
    ) -> None:
        self.indent = indent
        self.namespaces = namespaces
        self.ns_map = ns_map

    def render(self, obj: object) -> str:
        """Render a model instance into an XML string."""
        return serialize(
            obj,
            indent=self.indent,
            namespaces=self.namespaces,
            ns_map=self.ns_map,
        ).decode("utf-8")


class XmlParser:
    """Drop-in xsdata-compatible XML parser."""

    def __init__(self, **_kwargs: object) -> None:
        pass

    def from_string[T](self, source: str, clazz: type[T]) -> T:
        """Parse an XML string into a model instance."""
        return deserialize(source, clazz)

    def from_bytes[T](self, source: bytes, clazz: type[T]) -> T:
        """Parse XML bytes into a model instance."""
        return deserialize(source, clazz)

    def parse[T](
        self, source: bytes | str | pathlib.Path | IO[bytes] | IO[str], clazz: type[T]
    ) -> T:
        """Parse an XML source into a model instance."""
        return deserialize(source, clazz)


try:
    import msgspec

    _HAS_MSGSPEC = True
except ImportError:  # pragma: no cover
    msgspec = None  # type: ignore[assignment]
    _HAS_MSGSPEC = False

_CLASS_CACHE: dict[str, type[object]] = {}


def _resolve_class(class_identifier: str) -> type[object] | None:
    if not class_identifier:
        return None
    if class_identifier in _CLASS_CACHE:
        return _CLASS_CACHE[class_identifier]
    if ":" in class_identifier:
        module_name, class_name = class_identifier.rsplit(":", 1)
        try:
            mod = importlib.import_module(module_name)
            resolved = getattr(mod, class_name)
            _CLASS_CACHE[class_identifier] = resolved
            return resolved
        except (ImportError, AttributeError):
            return None
    return None


def _enc_hook(obj: object) -> object:
    if isinstance(obj, pathlib.Path | Decimal):
        return str(obj)
    if isinstance(obj, Enum):
        return obj.value
    if isinstance(obj, QName):
        return str(obj)
    if isinstance(obj, UserString):
        return str(obj)
    if hasattr(obj, "model_dump") and callable(obj.model_dump):
        return obj.model_dump()
    if hasattr(obj, "from_string"):
        return str(obj)
    raise NotImplementedError(
        f"PolyXML binary serializer cannot serialize object of type {type(obj).__name__}"
    )


def _dec_hook(target_type: type[object], obj: object) -> object:
    if target_type is pathlib.Path:
        return pathlib.Path(str(obj))
    if target_type is Decimal:
        return Decimal(str(obj))
    if isinstance(target_type, type) and issubclass(target_type, Enum):
        return target_type(obj)
    if target_type is QName or (isinstance(target_type, type) and issubclass(target_type, QName)):
        return QName(str(obj))
    if hasattr(target_type, "model_validate") and callable(target_type.model_validate):
        return target_type.model_validate(obj)
    if hasattr(target_type, "from_string"):
        return target_type.from_string(str(obj))
    if isinstance(target_type, type) and issubclass(target_type, UserString):
        return target_type(str(obj))
    raise NotImplementedError(
        f"PolyXML binary deserializer cannot deserialize object into type {target_type.__name__}"
    )


def dumps_binary(obj: object, *, tag_class: bool = False) -> bytes:
    """Serialize a model or dataclass into a high-throughput binary MessagePack buffer.

    Args:
        obj: Python dataclass, Pydantic model, or object.
        tag_class: When True, embeds the class identifier in the binary payload
            allowing loads_binary to reconstruct the exact model class dynamically.

    Returns:
        Compact binary MessagePack bytes.

    Raises:
        RuntimeError: If msgspec is not installed.
    """
    if not _HAS_MSGSPEC:  # pragma: no cover
        raise RuntimeError(
            "PolyXML binary serialization requires 'msgspec'. "
            "Install it via: pip install 'polyxml[msgpack]' or pip install msgspec"
        )
    if tag_class:
        if is_dataclass(obj) or hasattr(obj.__class__, "model_fields"):
            cls = obj.__class__
            identifier = f"{cls.__module__}:{cls.__qualname__}"
            _CLASS_CACHE[identifier] = cls
            payload = msgspec.msgpack.encode(obj, enc_hook=_enc_hook)
            return msgspec.msgpack.encode((identifier, payload))
        payload = msgspec.msgpack.encode(obj, enc_hook=_enc_hook)
        return msgspec.msgpack.encode(("", payload))

    return msgspec.msgpack.encode(obj, enc_hook=_enc_hook)


def loads_binary[T](data: bytes, target_type: type[T] | None = None) -> T | object:
    """Deserialize binary MessagePack bytes into a strongly-typed model or object.

    Args:
        data: Binary MessagePack bytes to decode.
        target_type: Optional target dataclass or model class. If omitted and the
            payload was serialized with tag_class=True, dynamically reconstructs the
            original model instance. Otherwise returns a dynamic dictionary/structure.

    Returns:
        The deserialized model instance or object.

    Raises:
        RuntimeError: If msgspec is not installed.
    """
    if not _HAS_MSGSPEC:  # pragma: no cover
        raise RuntimeError(
            "PolyXML binary deserialization requires 'msgspec'. "
            "Install it via: pip install 'polyxml[msgpack]' or pip install msgspec"
        )
    if target_type is not None:
        try:
            return msgspec.msgpack.decode(data, type=target_type, dec_hook=_dec_hook)
        except (msgspec.DecodeError, msgspec.ValidationError):
            try:
                decoded = msgspec.msgpack.decode(data)
                if (
                    isinstance(decoded, (list, tuple))
                    and len(decoded) == 2
                    and isinstance(decoded[0], str)
                    and isinstance(decoded[1], (bytes, bytearray))
                ):
                    return msgspec.msgpack.decode(decoded[1], type=target_type, dec_hook=_dec_hook)
            except Exception:
                pass
            raise

    try:
        decoded = msgspec.msgpack.decode(data)
        if (
            isinstance(decoded, (list, tuple))
            and len(decoded) == 2
            and isinstance(decoded[0], str)
            and isinstance(decoded[1], (bytes, bytearray))
        ):
            tag, payload = decoded
            if tag:
                resolved_cls = _resolve_class(tag)
                if resolved_cls is not None:
                    return msgspec.msgpack.decode(payload, type=resolved_cls, dec_hook=_dec_hook)
            return msgspec.msgpack.decode(payload, dec_hook=_dec_hook)
        return decoded
    except Exception:
        return msgspec.msgpack.decode(data, dec_hook=_dec_hook)


__all__ = [
    "AnyElement",
    "JsonParser",
    "JsonSerializer",
    "XmlParser",
    "XmlSerializer",
    "__version__",
    "deserialize",
    "deserialize_json",
    "dumps_binary",
    "dumps_json",
    "iterparse",
    "json_to_xml",
    "loads_binary",
    "loads_json",
    "serialize",
    "serialize_json",
    "xml_to_json",
]
