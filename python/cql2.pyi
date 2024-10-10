from typing import Any
from os import PathLike

class SqlQuery:
    """A SQL query"""

    query: str
    """The query, with parameterized fields."""

    params: list[str]
    """The parameters, to use for binding."""

class Expr:
    @staticmethod
    def from_path(path: PathLike | str) -> Expr:
        """Reads CQL2 from a filesystem path.

        Args:
            path (PathLike | str): The input path

        Returns:
            Expr: The CQL2 expression

        Examples:
            >>> from cql2 import Expr
            >>> expr = Expr.from_path("fixtures/text/example01.txt")
        """

    def __init__(self, cql2: str | dict[str, Any]) -> None:
        """A CQL2 expression.

        The cql2 can either be a cql2-text string, a cql2-json string, or a
        cql2-json dictionary.

        Args:
            cql2 (str | dict[str, Any]): The input CQL2

        Examples:
            >>> from cql2 import Expr
            >>> expr = Expr("landsat:scene_id = 'LC82030282019133LGN00'")
            >>> expr = Expr({"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]})
        """

    def validate(self) -> None:
        """Validates this expression using json-schema.

        Raises:
            ValidationError: Raised if the validation fails

        Examples:
            >>> from cql2 import Expr
            >>> expr = Expr("landsat:scene_id = 'LC82030282019133LGN00'")
            >>> expr.validate()
        """

    def to_json(self) -> dict[str, Any]:
        """Converts this cql2 expression to a cql2-json dictionary.

        Returns:
            dict[str, Any]: The cql2-json

        Examples:
            >>> from cql2 import Expr
            >>> expr = Expr("landsat:scene_id = 'LC82030282019133LGN00'")
            >>> expr.to_json()
            {'op': '=', 'args': [{'property': 'landsat:scene_id'}, 'LC82030282019133LGN00']}
        """

    def to_text(self) -> str:
        """Converts this cql2 expression to cql2-text.

        Returns:
            str: The cql2-text

        Examples:
            >>> from cql2 import Expr
            >>> expr = Expr({"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]})
            >>> expr.to_text()
            '("landsat:scene_id" = \'LC82030282019133LGN00\')'
        """

    def to_sql(self) -> SqlQuery:
        """Converts this cql2 expression to a SQL query.

        Returns:
            SqlQuery: The SQL query and parameters

        Examples:
            >>> from cql2 import Expr
            >>> expr = Expr("landsat:scene_id = 'LC82030282019133LGN00'")
            >>> q.query
            '("landsat:scene_id" = $1)'
            >>> q.params
            ['LC82030282019133LGN00']
        """

class ValidationError(Exception):
    """An error raised when cql2 json-schema validation fails."""
