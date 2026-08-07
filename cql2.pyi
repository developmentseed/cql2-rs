from typing import Any
from os import PathLike

__version__: str

def main() -> None:
    """Runs the cql2 command-line interface.

    This is the entry point behind the ``cql2`` console script; it reads its
    arguments from ``sys.argv``.
    """

def parse_file(path: PathLike | str) -> Expr:
    """Parses CQL2 from a filesystem path.

    Args:
        path (PathLike | str): The input path

    Returns:
        Expr: The CQL2 expression

    Examples:
        >>> from cql2 import parse_file
        >>> expr = parse_file("examples/text/example01.txt")
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
        >>> from cql2 import parse_text
        >>> expr = parse_text("landsat:scene_id = 'LC82030282019133LGN00'")
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
        >>> from cql2 import parse_json
        >>> expr = parse_json('{"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]}')
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
            "landsat:scene_id = 'LC82030282019133LGN00'"
        """

    def to_sql(self) -> str:
        r"""Converts this cql2 expression to a SQL query.

        Returns:
            str: The SQL query

        Examples:
            >>> from cql2 import Expr
            >>> expr = Expr("landsat:scene_id = 'LC82030282019133LGN00'")
            >>> expr.to_sql()
            '"landsat:scene_id" = \'LC82030282019133LGN00\''
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

    def __eq__(self, other: object) -> bool:
        """Compares two cql2 expressions for structural equality.

        Comparing against anything that is not an ``Expr`` returns False.

        Args:
            other (object): The object to compare against

        Returns:
            bool: True if both are equivalent expressions, False otherwise

        Examples:
            >>> from cql2 import Expr
            >>> Expr("landsat:cloud_cover = 10") == Expr("landsat:cloud_cover = 10")
            True
        """

    def __str__(self) -> str:
        """Returns the cql2-text representation of this expression.

        Returns:
            str: The cql2-text

        Examples:
            >>> from cql2 import Expr
            >>> str(Expr("landsat:cloud_cover = 10"))
            'landsat:cloud_cover = 10'
        """

    def __repr__(self) -> str:
        """Returns a debugging representation of this expression.

        Returns:
            str: The representation

        Examples:
            >>> from cql2 import Expr
            >>> repr(Expr("landsat:cloud_cover = 10"))
            'Expr(landsat:cloud_cover = 10)'
        """

class ParseError(Exception):
    """An error raised when cql2 parsing fails."""

class ValidationError(Exception):
    """An error raised when cql2 json-schema validation fails."""
