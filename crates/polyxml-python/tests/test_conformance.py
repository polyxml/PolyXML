import pathlib
from dataclasses import dataclass, field

from pydantic import BaseModel, Field

import polyxml

FIXTURES_DIR = pathlib.Path(__file__).parents[3] / "tests" / "fixtures" / "samples"


# -------------------------------------------------------------
# 1. SOAP 1.2 Envelope (Dataclasses)
# -------------------------------------------------------------


@dataclass
class SoapSecurityToken:
    class Meta:
        name = "SecurityToken"
        namespace = "http://example.com/security"

    token: str = field(
        metadata={"type": "Text"},
        default="",
    )
    must_understand: bool = field(
        metadata={
            "name": "mustUnderstand",
            "type": "Attribute",
            "namespace": "http://www.w3.org/2003/05/soap-envelope",
        },
        default=False,
    )


@dataclass
class SoapHeader:
    class Meta:
        name = "Header"
        namespace = "http://www.w3.org/2003/05/soap-envelope"

    security_token: SoapSecurityToken = field(
        metadata={
            "name": "SecurityToken",
            "type": "Element",
            "namespace": "http://example.com/security",
        },
        default_factory=SoapSecurityToken,
    )


@dataclass
class GetStockPrice:
    class Meta:
        name = "GetStockPrice"
        namespace = "http://example.com/stock"

    stock_name: str = field(
        metadata={
            "name": "StockName",
            "type": "Element",
            "namespace": "http://example.com/stock",
        },
        default="",
    )
    currency: str = field(
        metadata={
            "name": "Currency",
            "type": "Element",
            "namespace": "http://example.com/stock",
        },
        default="",
    )
    target_date: str = field(
        metadata={
            "name": "TargetDate",
            "type": "Element",
            "namespace": "http://example.com/stock",
        },
        default="",
    )


@dataclass
class SoapBody:
    class Meta:
        name = "Body"
        namespace = "http://www.w3.org/2003/05/soap-envelope"

    get_stock_price: GetStockPrice = field(
        metadata={
            "name": "GetStockPrice",
            "type": "Element",
            "namespace": "http://example.com/stock",
        },
        default_factory=GetStockPrice,
    )


@dataclass
class SoapEnvelope:
    class Meta:
        name = "Envelope"
        namespace = "http://www.w3.org/2003/05/soap-envelope"

    header: SoapHeader = field(
        metadata={
            "name": "Header",
            "type": "Element",
            "namespace": "http://www.w3.org/2003/05/soap-envelope",
        },
        default_factory=SoapHeader,
    )
    body: SoapBody = field(
        metadata={
            "name": "Body",
            "type": "Element",
            "namespace": "http://www.w3.org/2003/05/soap-envelope",
        },
        default_factory=SoapBody,
    )


def test_soap_envelope_conformance():
    xml_path = FIXTURES_DIR / "soap_envelope.xml"
    xml_bytes = xml_path.read_bytes()

    env = polyxml.deserialize(xml_bytes, SoapEnvelope)
    assert env.header.security_token.must_understand is True
    assert env.header.security_token.token.strip() == "sec-tok-987654321"
    assert env.body.get_stock_price.stock_name == "ACME"
    assert env.body.get_stock_price.currency == "USD"
    assert env.body.get_stock_price.target_date == "2026-09-17"

    # Roundtrip serialization with prefix mapping
    ns_map = {
        "env": "http://www.w3.org/2003/05/soap-envelope",
        "m": "http://example.com/stock",
        "auth": "http://example.com/security",
    }
    serialized = polyxml.serialize(env, indent=4, ns_map=ns_map)
    xml_str = serialized.decode("utf-8")
    assert 'xmlns:env="http://www.w3.org/2003/05/soap-envelope"' in xml_str
    assert "<env:Envelope" in xml_str
    assert "<env:Header>" in xml_str
    assert "<auth:SecurityToken" in xml_str

    re_env = polyxml.deserialize(serialized, SoapEnvelope)
    assert re_env.body.get_stock_price.stock_name == "ACME"
    assert re_env.header.security_token.must_understand is True


# -------------------------------------------------------------
# 2. Atom 1.0 Feed (Pydantic v2)
# -------------------------------------------------------------


class AtomAuthor(BaseModel):
    class Meta:
        name = "author"
        namespace = "http://www.w3.org/2005/Atom"

    name: str = Field(
        json_schema_extra={
            "name": "name",
            "type": "Element",
            "namespace": "http://www.w3.org/2005/Atom",
        }
    )
    email: str = Field(
        json_schema_extra={
            "name": "email",
            "type": "Element",
            "namespace": "http://www.w3.org/2005/Atom",
        }
    )


class AtomEntry(BaseModel):
    class Meta:
        name = "entry"
        namespace = "http://www.w3.org/2005/Atom"

    title: str = Field(
        json_schema_extra={
            "name": "title",
            "type": "Element",
            "namespace": "http://www.w3.org/2005/Atom",
        }
    )
    id: str = Field(
        json_schema_extra={
            "name": "id",
            "type": "Element",
            "namespace": "http://www.w3.org/2005/Atom",
        }
    )
    updated: str = Field(
        json_schema_extra={
            "name": "updated",
            "type": "Element",
            "namespace": "http://www.w3.org/2005/Atom",
        }
    )
    summary: str = Field(
        json_schema_extra={
            "name": "summary",
            "type": "Element",
            "namespace": "http://www.w3.org/2005/Atom",
        }
    )
    author: AtomAuthor = Field(
        json_schema_extra={
            "name": "author",
            "type": "Element",
            "namespace": "http://www.w3.org/2005/Atom",
        }
    )


class AtomFeed(BaseModel):
    class Meta:
        name = "feed"
        namespace = "http://www.w3.org/2005/Atom"

    title: str = Field(
        json_schema_extra={
            "name": "title",
            "type": "Element",
            "namespace": "http://www.w3.org/2005/Atom",
        }
    )
    id: str = Field(
        json_schema_extra={
            "name": "id",
            "type": "Element",
            "namespace": "http://www.w3.org/2005/Atom",
        }
    )
    updated: str = Field(
        json_schema_extra={
            "name": "updated",
            "type": "Element",
            "namespace": "http://www.w3.org/2005/Atom",
        }
    )
    entries: list[AtomEntry] = Field(
        default_factory=list,
        json_schema_extra={
            "name": "entry",
            "type": "Element",
            "namespace": "http://www.w3.org/2005/Atom",
        },
    )


def test_atom_feed_conformance():
    xml_path = FIXTURES_DIR / "atom_feed.xml"
    xml_text = xml_path.read_text(encoding="utf-8")

    feed = polyxml.deserialize(xml_text, AtomFeed)
    assert feed.title == "PolyXML Engineering Updates"
    assert len(feed.entries) == 2
    assert feed.entries[0].title == "Release v0.10.0"
    assert feed.entries[0].author.name == "Bailey Nguyen"
    assert feed.entries[1].title == "Performance Milestone"
    assert feed.entries[1].author.email == "core@polyxml.org"

    # Roundtrip serialize with default namespace
    ns_map = {None: "http://www.w3.org/2005/Atom"}
    serialized = polyxml.serialize(feed, indent=2, ns_map=ns_map)
    xml_str = serialized.decode("utf-8")
    assert 'xmlns="http://www.w3.org/2005/Atom"' in xml_str
    assert "<feed" in xml_str

    re_feed = polyxml.deserialize(serialized, AtomFeed)
    assert re_feed.title == feed.title
    assert len(re_feed.entries) == 2


# -------------------------------------------------------------
# 3. OASIS UBL 2.1 Invoice (Dataclasses)
# -------------------------------------------------------------


@dataclass
class UblParty:
    class Meta:
        name = "Party"
        namespace = "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"

    name: str = field(
        metadata={
            "name": "RegistrationName",
            "type": "Element",
            "namespace": "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2",
        },
        default="",
    )
    company_id: str = field(
        metadata={
            "name": "CompanyID",
            "type": "Element",
            "namespace": "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2",
        },
        default="",
    )


@dataclass
class UblSupplier:
    class Meta:
        name = "AccountingSupplierParty"
        namespace = "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"

    party: UblParty = field(
        metadata={
            "name": "Party",
            "type": "Element",
            "namespace": "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2",
        },
        default_factory=UblParty,
    )


@dataclass
class UblMonetaryTotal:
    class Meta:
        name = "LegalMonetaryTotal"
        namespace = "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"

    payable_amount: float = field(
        metadata={
            "name": "PayableAmount",
            "type": "Element",
            "namespace": "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2",
        },
        default=0.0,
    )


@dataclass
class UblInvoice:
    class Meta:
        name = "Invoice"
        namespace = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"

    id: str = field(
        metadata={
            "name": "ID",
            "type": "Element",
            "namespace": "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2",
        },
        default="",
    )
    issue_date: str = field(
        metadata={
            "name": "IssueDate",
            "type": "Element",
            "namespace": "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2",
        },
        default="",
    )
    currency: str = field(
        metadata={
            "name": "DocumentCurrencyCode",
            "type": "Element",
            "namespace": "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2",
        },
        default="",
    )
    supplier: UblSupplier = field(
        metadata={
            "name": "AccountingSupplierParty",
            "type": "Element",
            "namespace": "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2",
        },
        default_factory=UblSupplier,
    )
    totals: UblMonetaryTotal = field(
        metadata={
            "name": "LegalMonetaryTotal",
            "type": "Element",
            "namespace": "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2",
        },
        default_factory=UblMonetaryTotal,
    )


def test_ubl_invoice_conformance():
    xml_path = FIXTURES_DIR / "ubl_invoice.xml"
    xml_bytes = xml_path.read_bytes()

    inv = polyxml.deserialize(xml_bytes, UblInvoice)
    assert inv.id == "INV-2026-0042"
    assert inv.issue_date == "2026-09-17"
    assert inv.currency == "USD"
    assert inv.supplier.party.name == "PolyXML Tech Inc."
    assert inv.supplier.party.company_id == "US-987654321"
    assert inv.totals.payable_amount == 1620.0

    # Roundtrip serialize
    ns_map = {
        None: "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2",
        "cac": "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2",
        "cbc": "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2",
    }
    serialized = polyxml.serialize(inv, indent=2, ns_map=ns_map)
    xml_str = serialized.decode("utf-8")
    assert 'xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"' in xml_str
    assert (
        'xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"'
        in xml_str
    )
    assert (
        'xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"'
        in xml_str
    )

    re_inv = polyxml.deserialize(serialized, UblInvoice)
    assert re_inv.id == inv.id
    assert re_inv.totals.payable_amount == 1620.0


# -------------------------------------------------------------
# 4. Streaming Catalog Iterparse Conformance
# -------------------------------------------------------------


@dataclass
class CatalogItem:
    id: int = field(metadata={"name": "id", "type": "Attribute"}, default=0)
    name: str = field(metadata={"name": "Name", "type": "Element"}, default="")
    price: float = field(metadata={"name": "Price", "type": "Element"}, default=0.0)
    in_stock: bool = field(metadata={"name": "InStock", "type": "Element"}, default=False)


def test_streaming_catalog_iterparse_conformance():
    xml_path = FIXTURES_DIR / "streaming_catalog.xml"

    count = 0
    first_item = None
    mid_item = None
    last_item = None

    for item in polyxml.iterparse(xml_path, CatalogItem, tag="Item"):
        count += 1
        if count == 1:
            first_item = item
        elif count == 500:
            mid_item = item
        elif count == 1000:
            last_item = item

    assert count == 1000
    assert first_item is not None
    assert first_item.id == 1
    assert first_item.name == "Widget #1"
    assert first_item.price == 1.50
    assert first_item.in_stock is False

    assert mid_item is not None
    assert mid_item.id == 500
    assert mid_item.name == "Widget #500"
    assert mid_item.price == 750.0
    assert mid_item.in_stock is True

    assert last_item is not None
    assert last_item.id == 1000
    assert last_item.name == "Widget #1000"
    assert last_item.price == 1500.0
    assert last_item.in_stock is True
