"""Shared dataclass models for Python binary serialization benchmarks."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(slots=True)
class SensorReading:
    id: str
    temperature: float
    humidity: float
    pressure: float
    status: str
    timestamp: str


@dataclass(slots=True)
class SensorBatch:
    batch_id: str
    readings: list[SensorReading] = field(default_factory=list)


@dataclass(slots=True)
class OrderItem:
    sku: str
    quantity: int
    unit_price: float
    notes: str | None = None


@dataclass(slots=True)
class Order:
    order_id: str
    customer_id: str
    created_at: str
    priority: str = "NORMAL"
    discount_code: str | None = None
    items: list[OrderItem] = field(default_factory=list)


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
