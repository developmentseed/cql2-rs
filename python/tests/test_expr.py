import json
from pathlib import Path
from typing import Any

import pytest
import cql2
from cql2 import Expr, ParseError, ValidationError


def test_parse_file(fixtures: Path) -> None:
    cql2.parse_file(fixtures / "text" / "example01.txt")


def test_parse_file_str(fixtures: Path) -> None:
    cql2.parse_file(str(fixtures / "text" / "example01.txt"))


def test_init(example01_text: str) -> None:
    Expr(example01_text)


def test_parse_json(example01_text: str, example01_json: dict[str, Any]) -> None:
    cql2.parse_json(json.dumps(example01_json))
    with pytest.raises(ParseError):
        cql2.parse_json(example01_text)


def test_parse_text(example01_text: str, example01_json: dict[str, Any]) -> None:
    cql2.parse_text(example01_text)
    with pytest.raises(ParseError):
        cql2.parse_text(json.dumps(example01_json))


def test_to_json(example01_text: str) -> None:
    Expr(example01_text).to_json() == {
        "op": "=",
        "args": [{"property": "landsat:scene_id"}, "LC82030282019133LGN00"],
    }


def test_to_text(example01_json: dict[str, Any]) -> None:
    Expr(example01_json).to_text() == "landsat:scene_id = 'LC82030282019133LGN00'"


def test_to_sql(example01_text: str) -> None:
    sql_query = Expr(example01_text).to_sql()
    assert sql_query.query == '("landsat:scene_id" = $1)'
    assert sql_query.params == ["LC82030282019133LGN00"]


def test_validate() -> None:
    expr = Expr(
        {
            "op": "t_before",
            "args": [{"property": "updated_at"}, {"timestamp": "invalid-timestamp"}],
        }
    )
    with pytest.raises(ValidationError):
        expr.validate()
