"""Shared dataclass models for the Python JSON benchmark suite.

Covers:
  1. Small sensor telemetry reading (~170 bytes JSON).
  2. Nested order with attributes, items, and optional fields (~1.2 KB JSON).
  3. Moderate batch of 100 sensors (~17 KB JSON).
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(slots=True)
class SensorReading:
    """Telemetry reading from an edge or aerospace sensor."""

    id: str = field(metadata={"name": "id"})
    temperature: float = field(metadata={"name": "temperature"})
    humidity: float = field(metadata={"name": "humidity"})
    pressure: float = field(metadata={"name": "pressure"})
    status: str = field(metadata={"name": "status"})
    timestamp: str = field(metadata={"name": "timestamp"})


@dataclass(slots=True)
class SensorBatch:
    """Batch collection of multiple sensor readings."""

    batch_id: str = field(metadata={"name": "batch_id"})
    readings: list[SensorReading] = field(
        default_factory=list, metadata={"name": "readings"}
    )


@dataclass(slots=True)
class OrderItem:
    """Individual line item in an order."""

    sku: str = field(metadata={"name": "sku"})
    quantity: int = field(metadata={"name": "quantity"})
    unit_price: float = field(metadata={"name": "unit_price"})
    notes: str | None = field(default=None, metadata={"name": "notes"})


@dataclass(slots=True)
class Order:
    """Nested order model with optional customer metadata and repeated items."""

    order_id: str = field(metadata={"name": "order_id"})
    customer_id: str = field(metadata={"name": "customer_id"})
    created_at: str = field(metadata={"name": "created_at"})
    priority: str = field(default="NORMAL", metadata={"name": "priority"})
    discount_code: str | None = field(default=None, metadata={"name": "discount_code"})
    items: list[OrderItem] = field(default_factory=list, metadata={"name": "items"})


def make_sample_sensor() -> SensorReading:
    return SensorReading(
        id="sensor-0042",
        temperature=21.85,
        humidity=48.2,
        pressure=1013.25,
        status="ACTIVE",
        timestamp="2026-09-26T12:00:00Z",
    )


def make_sample_batch(count: int = 100) -> SensorBatch:
    readings = [
        SensorReading(
            id=f"sensor-{i:04d}",
            temperature=20.0 + (i % 15) * 0.5,
            humidity=40.0 + (i % 30) * 0.8,
            pressure=1000.0 + (i % 25) * 1.1,
            status="ACTIVE" if i % 10 != 0 else "DEGRADED",
            timestamp="2026-09-26T12:00:00Z",
        )
        for i in range(count)
    ]
    return SensorBatch(batch_id="batch-0001", readings=readings)


def make_sample_order(item_count: int = 10) -> Order:
    items = [
        OrderItem(
            sku=f"SKU-{1000 + i}",
            quantity=1 + (i % 5),
            unit_price=round(19.99 + i * 2.5, 2),
            notes="Rush delivery" if i == 0 else None,
        )
        for i in range(item_count)
    ]
    return Order(
        order_id="ORD-987654",
        customer_id="CUST-1029",
        created_at="2026-09-26T14:30:00Z",
        priority="HIGH",
        discount_code="FALL2026",
        items=items,
    )
