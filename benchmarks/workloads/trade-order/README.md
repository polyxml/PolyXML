# Moderate Trade-Order XML Workload

A representative, moderate XML workload featuring nested elements, XML attributes, repeated sub-elements, and documented optional-field distributions.

## Workload Specification

- **Schema**: `order.xsd` (Order feed with nested `Party`, `Item` list, and optional `Settlement`).
- **Nesting Depth**: 4 levels (`OrderFeed` -> `Order` -> `Item` -> text).
- **Element Count**: 6 complex types, 12 distinct XML elements.
- **Attribute Count**: 7 XML attributes across 4 elements (`feedId`, `timestamp`, `orderId`, `currency`, `priority`, `partyId`, `sku`, `category`).
- **Fixtures**:
  - `order-single.xml`: 383 bytes (1 order, 2 items; small-message latency baseline).
  - `order-feed-50.xml`: 21,330 bytes (50 orders, 117 items; moderate repeated-record throughput baseline).

### Documented Distribution of Optional Fields (Deterministic PRNG seed 42)

| Field | Location | Type | Presence Frequency |
| :--- | :--- | :--- | :--- |
| `priority` | `Order` attribute | `xs:int` | 60% (30 / 50 orders) |
| `Settlement` | `Order` child | `SettlementType` | 70% (35 / 50 orders) |
| `TaxId` | `Party` child | `xs:string` | 40% (20 / 50 parties) |
| `category` | `Item` attribute | `xs:string` | 50% (58 / 117 items) |
| `Discount` | `Item` child | `xs:double` | 35% (41 / 117 items) |
| `Notes` | `Item` child | `xs:string` | 25% (29 / 117 items) |

## Generation

Run `./generate_fixtures.py` to regenerate the fixtures deterministically.
