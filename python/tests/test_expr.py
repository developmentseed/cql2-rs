import json
from pathlib import Path
from typing import Any

import cql2
import pytest


def test_parse_file(fixtures: Path) -> None:
    cql2.parse_file(fixtures / "text" / "example01.txt")


def test_parse_file_str(fixtures: Path) -> None:
    cql2.parse_file(str(fixtures / "text" / "example01.txt"))


def test_init(example01_text: str) -> None:
    cql2.Expr(example01_text)


def test_parse_json(example01_text: str, example01_json: dict[str, Any]) -> None:
    cql2.parse_json(json.dumps(example01_json))
    with pytest.raises(cql2.ParseError):
        cql2.parse_json(example01_text)


def test_parse_text(example01_text: str, example01_json: dict[str, Any]) -> None:
    cql2.parse_text(example01_text)
    with pytest.raises(cql2.ParseError):
        cql2.parse_text(json.dumps(example01_json))


def test_to_json(example01_text: str) -> None:
    cql2.Expr(example01_text).to_json() == {
        "op": "=",
        "args": [{"property": "landsat:scene_id"}, "LC82030282019133LGN00"],
    }


def test_to_text(example01_json: dict[str, Any]) -> None:
    cql2.Expr(example01_json).to_text() == "landsat:scene_id = 'LC82030282019133LGN00'"


def test_to_sql(example01_text: str) -> None:
    sql_query = cql2.Expr(example01_text).to_sql()
    assert sql_query.query == '("landsat:scene_id" = $1)'
    assert sql_query.params == ["LC82030282019133LGN00"]


def test_validate() -> None:
    expr = cql2.Expr(
        {
            "op": "t_before",
            "args": [{"property": "updated_at"}, {"timestamp": "invalid-timestamp"}],
        }
    )
    with pytest.raises(cql2.ValidationError):
        expr.validate()


def test_add() -> None:
    assert cql2.Expr("True") + cql2.Expr("false") == cql2.Expr("true AND false")


def test_eq() -> None:
    assert cql2.Expr("True") == cql2.Expr("true")


@pytest.mark.parametrize(
    "expr, item, should_match",
    [
        pytest.param(
            "boolfield and 1 + 2 = 3",
            {
                "properties": {
                    "eo:cloud_cover": 10,
                    "datetime": "2020-01-01 00:00:00Z",
                    "boolfield": True,
                }
            },
            True,
            id="pass on bool & cql2 arithmetic",
        ),
        pytest.param(
            "eo:cloud_cover <= 9",
            {
                "properties": {
                    "eo:cloud_cover": 10,
                    "datetime": "2020-01-01 00:00:00Z",
                },
            },
            False,
            id="fail on property value comparison",
        ),
        pytest.param(
            "eo:cloud_cover <= 9",
            {
                "properties": {
                    "eo:cloud_cover": 8,
                    "datetime": "2020-01-01 00:00:00Z",
                },
            },
            True,
            id="pass on property value comparison",
        ),
    ],
)
def test_matches(expr, item, should_match) -> None:
    assert cql2.Expr(expr).matches(item) == should_match
