"""Dataclass models for Python AOT vs Dataclass benchmark.

Includes:
  1. Synthetic SensorReadingDataclass (slots=True)
  2. Fallback / direct UCI Entity model definitions for PolyXML dataclass deserialization
"""

from __future__ import annotations

import sys
from dataclasses import dataclass

# Try importing the generated UCI dataclass from polyxml-defense-examples if available
try:
    sys.path.insert(0, "/home/xenah/github/polyxml-defense-examples")
    from generated.python.uci_entity_core import EntityMt as UciDataclassEntityMt
except (ImportError, ModuleNotFoundError):
    UciDataclassEntityMt = None


@dataclass(slots=True)
class SensorReadingDataclass:
    sensorId: str
    temperature: float
    humidity: float
    pressure: float
    status: str
