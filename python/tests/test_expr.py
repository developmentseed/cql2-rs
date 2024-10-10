from pathlib import Path

import pytest
from cql2 import Expr, ValidationError


def test_from_path(fixtures: Path) -> None:
    Expr.from_path(fixtures / "text" / "example01.txt")


def test_from_path_str(fixtures: Path) -> None:
    Expr.from_path(str(fixtures / "text" / "example01.txt"))


def test_init(example01_text: str) -> None:
    Expr(example01_text)


def test_to_json(example01_text: str) -> None:
    Expr(example01_text).to_json() == {
        "op": "=",
        "args": [{"property": "landsat:scene_id"}, "LC82030282019133LGN00"],
    }


def test_to_text(example01_json: str) -> None:
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
