#!/usr/bin/env python3
"""Deterministic fixture generator for the moderate trade-order XML workload.

Generates:
  - order-single.xml: 1 order, ~700 B (small-message case)
  - order-feed-50.xml: 50 orders, ~35 KB (moderate repeated-record case)

Documented distribution of optional fields (seeded with PRNG seed 42):
  - Order.priority: 60% present
  - Party.TaxId: 40% present
  - Item.Discount: 35% present
  - Item.Notes: 25% present
  - Item.category: 50% present
  - Order.Settlement: 70% present
"""

import random
from pathlib import Path

HERE = Path(__file__).resolve().parent

def build_order_xml(order_idx: int, rng: random.Random) -> str:
    has_priority = rng.random() < 0.60
    priority_attr = f' priority="{rng.randint(1, 5)}"' if has_priority else ""
    currency = rng.choice(["USD", "EUR", "GBP", "JPY"])
    
    party_id = f"PTY-{1000 + order_idx % 20}"
    party_name = f"Trading Corp {order_idx % 20}"
    has_tax_id = rng.random() < 0.40
    tax_elem = f"<TaxId>TAX-{5000 + order_idx % 20}</TaxId>" if has_tax_id else ""
    party_xml = f'<Party partyId="{party_id}"><Name>{party_name}</Name>{tax_elem}</Party>'

    item_count = rng.randint(1, 4)
    items_xml = []
    for item_idx in range(item_count):
        sku = f"SKU-{100 + (order_idx * 7 + item_idx) % 50}"
        has_cat = rng.random() < 0.50
        cat_attr = f' category="{rng.choice(["EQUITY", "FX", "BOND", "COMMODITY"])}"' if has_cat else ""
        qty = rng.randint(10, 500)
        unit_price = round(rng.uniform(10.0, 500.0), 2)
        has_discount = rng.random() < 0.35
        discount_elem = f"<Discount>{round(rng.uniform(0.5, 15.0), 2)}</Discount>" if has_discount else ""
        has_notes = rng.random() < 0.25
        notes_elem = f"<Notes>Execution instruction {item_idx}</Notes>" if has_notes else ""
        
        item_xml = (
            f'<Item sku="{sku}"{cat_attr}>'
            f'<Quantity>{qty}</Quantity>'
            f'<UnitPrice>{unit_price}</UnitPrice>'
            f'{discount_elem}'
            f'{notes_elem}'
            f'</Item>'
        )
        items_xml.append(item_xml)

    has_settlement = rng.random() < 0.70
    if has_settlement:
        status = rng.choice(["PENDING", "SETTLED", "CLEARED"])
        venue = rng.choice(["XNYS", "XNAS", "XLON", "XFRA"])
        settlement_xml = f'<Settlement><Status>{status}</Status><Venue>{venue}</Venue></Settlement>'
    else:
        settlement_xml = ""

    return (
        f'<Order orderId="ORD-{order_idx:05d}" currency="{currency}"{priority_attr}>'
        f'{party_xml}'
        f'{"".join(items_xml)}'
        f'{settlement_xml}'
        f'</Order>'
    )

def main() -> None:
    # 1. Single order
    rng_single = random.Random(42)
    single_order = build_order_xml(0, rng_single)
    single_feed = (
        f'<OrderFeed feedId="FEED-SINGLE" timestamp="2026-09-26T12:00:00Z">'
        f'{single_order}'
        f'</OrderFeed>'
    )
    (HERE / "order-single.xml").write_text(single_feed, encoding="utf-8")

    # 2. Feed with 50 orders
    rng_feed = random.Random(42)
    orders = [build_order_xml(i, rng_feed) for i in range(50)]
    feed_50 = (
        f'<OrderFeed feedId="FEED-BATCH-50" timestamp="2026-09-26T12:00:00Z">'
        f'{"".join(orders)}'
        f'</OrderFeed>'
    )
    (HERE / "order-feed-50.xml").write_text(feed_50, encoding="utf-8")

    print(f"Generated order-single.xml: {len(single_feed.encode('utf-8'))} bytes")
    print(f"Generated order-feed-50.xml: {len(feed_50.encode('utf-8'))} bytes")

if __name__ == "__main__":
    main()
