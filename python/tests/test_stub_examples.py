import ast
import doctest
from pathlib import Path

import pytest

ROOT = Path(__file__).parents[2]


def test_stub_examples(monkeypatch: pytest.MonkeyPatch) -> None:
    """The examples in the docstrings of `cql2.pyi` give the output they show."""
    monkeypatch.chdir(ROOT)
    parser = doctest.DocTestParser()
    runner = doctest.DocTestRunner()
    examples = 0
    for node in ast.walk(ast.parse((ROOT / "cql2.pyi").read_text())):
        if not isinstance(node, (ast.FunctionDef, ast.ClassDef)):
            continue
        docstring = ast.get_docstring(node)
        if docstring:
            test = parser.get_doctest(docstring, {}, node.name, "cql2.pyi", 0)
            examples += len(test.examples)
            runner.run(test)
    assert examples > 0
    assert runner.failures == 0
