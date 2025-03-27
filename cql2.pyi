from typing import Any
from os import PathLike

def parse_file(path: PathLike | str) -> Expr:
    """Parses CQL2 from a filesystem path.

    Args:
        path (PathLike | str): The input path

    Returns:
        Expr: The CQL2 expression

    Examples:
        >>> from cql2 import Expr
        >>> expr = Expr.parse_file("fixtures/text/example01.txt")
    """

def parse_text(s: str) -> Expr:
    """Parses cql2-text.

    Args:
        s (str): The cql2-text

    Returns:
        Expr: The CQL2 expression

    Raises:
        ParseError: Raised if the string does not parse as cql2-text

    Examples:
        >>> from cql2 import Expr
        >>> expr = Expr.parse_text("landsat:scene_id = 'LC82030282019133LGN00'")
    """

def parse_json(s: str) -> Expr:
    """Parses cql2-json.

    Args:
        s (str): The cql2-json string

    Returns:
        Expr: The CQL2 expression

    Raises:
        ParseError: Raised if the string does not parse as cql2-json

    Examples:
        >>> from cql2 import Expr
        >>> expr = Expr.parse_json('{"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]}')
    """

class Expr:
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

    def matches(self, item: dict[str, Any]) -> bool:
        """Matches this expression against an item.

        Args:
            item (dict[str, Any]): The item to match against

        Returns:
            bool: True if the expression matches the item, False otherwise
        """

    def reduce(self, item: dict[str, Any] | None = None) -> Expr:
        """Reduces this expression against an item.

        Args:
            item (dict[str, Any] | None): The item to reduce against

        Returns:
            Expr: The reduced expression

        Examples:
            >>> from cql2 import Expr
            >>> expr = Expr("true AND true").reduce()
            >>> expr.to_text()
            'true'
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

    def __add__(self, other: Expr) -> Expr:
        """Combines two cql2 expressions using the AND operator.

        Args:
            other (Expr): The other expression

        Returns:
            Expr: The combined expression

        Examples:
            >>> from cql2 import Expr
            >>> expr1 = Expr("landsat:scene_id = 'LC82030282019133LGN00'")
            >>> expr2 = Expr("landsat:cloud_cover = 10")
            >>> expr = expr1 + expr2
        """

class SqlQuery:
    """A SQL query"""

    query: str
    """The query, with parameterized fields."""

    params: list[str]
    """The parameters, to use for binding."""

class ParseError(Exception):
    """An error raised when cql2 parsing fails."""

class ValidationError(Exception):
    """An error raised when cql2 json-schema validation fails."""
