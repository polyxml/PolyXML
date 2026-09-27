from dataclasses import dataclass, field

import polyxml


@dataclass(slots=True, kw_only=True)
class Card:
    class Meta:
        name = "Card"
        namespace = "urn:polyxml:mixed"

    title: str = field(metadata={"type": "Element", "name": "title"})
    code: str | None = field(default=None, metadata={"type": "Attribute", "name": "code"})


@dataclass(slots=True, kw_only=True)
class RichTextItem:
    kind: str
    value: Card | str


@dataclass(slots=True, kw_only=True)
class RichText:
    class Meta:
        name = "RichText"
        namespace = "urn:polyxml:mixed"
        mixed_branches = (
            ("Text", "#text", None, "str"),
            ("b", "b", "urn:polyxml:mixed", "str"),
            ("card", "card", "urn:polyxml:mixed", "Card"),
        )

    lang: str | None = field(default=None, metadata={"type": "Attribute", "name": "lang"})
    items: list[RichTextItem] = field(
        default_factory=list, metadata={"type": "Element", "name": ""}
    )


def test_mixed_text_and_nested_child_roundtrip() -> None:
    xml = b'<RichText lang="en">open <b>bold</b> then <card code="c1"><title>One</title></card> close</RichText>'
    value = polyxml.deserialize(xml, RichText)
    assert [item.kind for item in value.items] == ["#text", "b", "#text", "card", "#text"]
    assert value.items[3].value == Card(title="One", code="c1")
    assert polyxml.serialize(value, namespaces=False) == xml
