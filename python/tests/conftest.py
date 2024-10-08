import pytest
import json
from typing import Any
from pathlib import Path


@pytest.fixture
def fixtures() -> Path:
    return Path(__file__).parents[2] / "fixtures"


@pytest.fixture
def example01_text(fixtures: Path) -> str:
    with open(fixtures / "text" / "example01.txt") as f:
        return f.read()


@pytest.fixture
def example01_json(fixtures: Path) -> dict[str, Any]:
    with open(fixtures / "json" / "example01.json") as f:
        return json.load(f)
