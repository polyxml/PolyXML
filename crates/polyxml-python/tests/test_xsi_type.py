"""xsi:type polymorphic dispatch through the Python runtime.

Covers both directions: parsing wire xsi:type selectors into concrete
subclasses (discovered from the Python class hierarchy), and re-emitting
those selectors on serialize so round trips preserve the concrete type.
"""

import json
from dataclasses import dataclass, field

import pytest
from pydantic import BaseModel, Field

import polyxml

# ---------------------------------------------------------------
# Dataclass fixtures: abstract Vehicle base with Car/Truck derivations
# ---------------------------------------------------------------


@dataclass
class Vehicle:
    class Meta:
        name = "Vehicle"
        abstract = True

    id: str = field(metadata={"type": "Element", "name": "id"})


@dataclass
class Car(Vehicle):
    class Meta:
        name = "Car"

    doors: int = field(metadata={"type": "Element", "name": "doors"})


@dataclass
class Truck(Vehicle):
    class Meta:
        name = "Truck"

    payload: str = field(metadata={"type": "Element", "name": "payload"})


@dataclass
class Fleet:
    class Meta:
        name = "Fleet"

    vehicle: Vehicle = field(
        metadata={"type": "Element", "name": "vehicle"},
        default_factory=Vehicle,
    )
    spares: list[Vehicle] = field(
        metadata={"type": "Element", "name": "spare"},
        default_factory=list,
    )


@dataclass
class NullableFleet:
    class Meta:
        name = "Fleet"

    vehicle: Vehicle | None = field(
        default=None,
        metadata={"type": "Element", "name": "vehicle", "nillable": True},
    )


@dataclass
class LonelyBase:
    class Meta:
        name = "LonelyBase"
        abstract = True

    id: str = field(metadata={"type": "Element", "name": "id"})


@dataclass
class Tagged:
    class Meta:
        name = "Tagged"

    label: str = field(metadata={"type": "Element", "name": "label"})
    type: str = field(metadata={"type": "Attribute", "name": "type"}, default="")


FLEET_XML = (
    b'<Fleet xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">'
    b'<vehicle xsi:type="Car"><id>V1</id><doors>4</doors></vehicle>'
    b'<spare xsi:type="Truck"><id>S1</id><payload>ore</payload></spare>'
    b'<spare xsi:type="Car"><id>S2</id><doors>2</doors></spare>'
    b"</Fleet>"
)


def test_nil_abstract_complex_type_without_selector():
    xml = (
        b'<Fleet xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">'
        b'<vehicle xsi:nil="true"/></Fleet>'
    )
    assert polyxml.deserialize(xml, NullableFleet).vehicle is None


def make_fleet() -> Fleet:
    return Fleet(
        vehicle=Car(id="V1", doors=4),
        spares=[
            Truck(id="S1", payload="ore"),
            Car(id="S2", doors=2),
        ],
    )


def test_deserialize_nested_xsi_type_dispatch():
    fleet = polyxml.deserialize(FLEET_XML, Fleet)

    assert type(fleet.vehicle) is Car
    assert fleet.vehicle.id == "V1"  # inherited base field
    assert fleet.vehicle.doors == 4

    assert type(fleet.spares) is list
    assert type(fleet.spares[0]) is Truck
    assert fleet.spares[0].payload == "ore"
    assert type(fleet.spares[1]) is Car
    assert fleet.spares[1].doors == 2


def test_nested_xsi_type_round_trip():
    fleet = make_fleet()

    out = polyxml.serialize(fleet)
    assert b'xsi:type="Car"' in out, out
    assert b'xsi:type="Truck"' in out, out
    assert b'xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"' in out, out

    reparsed = polyxml.deserialize(out, Fleet)
    assert reparsed == fleet


def test_root_xsi_type_dispatch():
    xml = b'<Vehicle xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:type="Truck"><id>T9</id><payload>ore</payload></Vehicle>'

    vehicle = polyxml.deserialize(xml, Vehicle)
    assert type(vehicle) is Truck
    assert vehicle.id == "T9"
    assert vehicle.payload == "ore"


def test_root_round_trip_with_target_type():
    truck = Truck(id="T9", payload="ore")

    out = polyxml.serialize(truck, target_type=Vehicle)
    assert out.startswith(b"<Vehicle"), out
    assert b'xsi:type="Truck"' in out, out

    reparsed = polyxml.deserialize(out, Vehicle)
    assert reparsed == truck


def test_concrete_serialize_without_target_type():
    car = Car(id="V1", doors=4)

    out = polyxml.serialize(car)
    assert out.startswith(b"<Car"), out
    assert b"xsi:type=" not in out, out


def test_unknown_xsi_type_raises():
    xml = b'<Fleet xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><vehicle xsi:type="Plane"><id>X</id></vehicle></Fleet>'

    with pytest.raises(ValueError, match="known derivation"):
        polyxml.deserialize(xml, Fleet)


def test_abstract_without_derivations_raises():
    xml = b'<LonelyBase xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:type="Other"><id>X</id></LonelyBase>'

    with pytest.raises(ValueError, match="no registered derivations"):
        polyxml.deserialize(xml, LonelyBase)

    with pytest.raises(ValueError, match="escape hatch"):
        polyxml.deserialize(xml, LonelyBase)


def test_plain_type_attribute_not_dispatched():
    # A content attribute literally named "type" on a non-abstract,
    # derivation-free model must never be treated as an xsi:type selector.
    res = polyxml.deserialize(b'<Tagged type="gear"><label>L</label></Tagged>', Tagged)
    assert res.type == "gear"
    assert res.label == "L"

    out = polyxml.serialize(res)
    assert b"xsi:type=" not in out, out
    assert b'type="gear"' in out, out


# ---------------------------------------------------------------
# Pydantic fixtures: discovery also walks BaseModel subclass trees
# ---------------------------------------------------------------


class PVehicle(BaseModel):
    id: str = Field(..., json_schema_extra={"type": "Element"})


class PCar(PVehicle):
    doors: int = Field(..., json_schema_extra={"type": "Element"})


class PTruck(PVehicle):
    payload: str = Field(..., json_schema_extra={"type": "Element"})


class PFleet(BaseModel):
    vehicle: PVehicle = Field(..., json_schema_extra={"type": "Element"})


def test_pydantic_xsi_type_dispatch_and_round_trip():
    xml = b'<PFleet xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><vehicle xsi:type="PCar"><id>V9</id><doors>2</doors></vehicle></PFleet>'

    fleet = polyxml.deserialize(xml, PFleet)
    assert type(fleet.vehicle) is PCar
    assert fleet.vehicle.id == "V9"
    assert fleet.vehicle.doors == 2

    out = polyxml.serialize(fleet)
    assert b'xsi:type="PCar"' in out, out

    reparsed = polyxml.deserialize(out, PFleet)
    assert reparsed == fleet


def test_iterparse_xsi_type_dispatch():
    xml = (
        b'<wrap xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">'
        b'<vehicle xsi:type="Car"><id>A</id><doors>2</doors></vehicle>'
        b'<vehicle xsi:type="Truck"><id>B</id><payload>p</payload></vehicle>'
        b"</wrap>"
    )

    items = list(polyxml.iterparse(xml, Vehicle, tag="vehicle"))
    assert [type(v) for v in items] == [Car, Truck]
    assert items[0].doors == 2
    assert items[1].payload == "p"


def test_xml_to_json_keeps_variant_fields():
    xml = b'<Fleet xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><vehicle xsi:type="Car"><id>V1</id><doors>4</doors></vehicle></Fleet>'

    out = polyxml.xml_to_json(xml, target_type=Fleet)
    data = json.loads(out)
    assert data["vehicle"]["id"] == "V1"
    assert data["vehicle"]["doors"] == 4


def test_subclass_defined_after_base_was_cached_dispatches():
    @dataclass
    class LateBase:
        class Meta:
            name = "LateBase"
            abstract = True

        id: str = field(metadata={"type": "Element", "name": "id"})

    @dataclass
    class LateFleet:
        class Meta:
            name = "LateFleet"

        vehicle: LateBase = field(metadata={"type": "Element", "name": "vehicle"})

    xml = b'<LateBase xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:type="LateChild"><id>X</id><extra>Y</extra></LateBase>'
    with pytest.raises(ValueError):
        polyxml.deserialize(xml, LateBase)
    fleet_xml = b'<LateFleet xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><vehicle xsi:type="LateChild"><id>X</id><extra>Y</extra></vehicle></LateFleet>'
    with pytest.raises(ValueError):
        polyxml.deserialize(fleet_xml, LateFleet)

    @dataclass
    class LateChild(LateBase):
        class Meta:
            name = "LateChild"

        extra: str = field(metadata={"type": "Element", "name": "extra"})

    value = polyxml.deserialize(xml, LateBase)
    assert isinstance(value, LateChild)
    assert value.extra == "Y"
    fleet = polyxml.deserialize(fleet_xml, LateFleet)
    assert isinstance(fleet.vehicle, LateChild)
