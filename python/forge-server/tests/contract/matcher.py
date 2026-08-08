"""``${name}`` substitution and the matcher the expectations are written in.

Objects match by subset, so a case asserts the fields it is about and stays
quiet about the rest; ``$exact`` opts back into deep equality. See
``contract/README.md`` for the full table.

This is the Python reading of the same rules ``crates/forge-contract`` reads in
Rust. The corpus is the shared thing; each language brings its own matcher.
"""

from __future__ import annotations

import json
from typing import Any, Mapping

Vars = dict[str, str]


class MatchError(AssertionError):
    """An expectation the response did not meet, or a corpus bug."""


def interpolate(text: str, variables: Mapping[str, str]) -> str:
    """Replace every ``${name}`` in ``text``.

    An unknown name is an error rather than an empty string: a typo in the
    corpus should fail loudly.
    """
    out: list[str] = []
    rest = text
    while True:
        start = rest.find("${")
        if start < 0:
            out.append(rest)
            return "".join(out)
        out.append(rest[:start])
        tail = rest[start + 2 :]
        end = tail.find("}")
        if end < 0:
            raise MatchError(f"unterminated ${{ in {text!r}")
        name = tail[:end]
        if name not in variables:
            raise MatchError(f"unknown variable ${{{name}}} in {text!r}")
        out.append(variables[name])
        rest = tail[end + 1 :]


def interpolate_value(value: Any, variables: Mapping[str, str]) -> Any:
    """Interpolate every string in a JSON value, keys included."""
    if isinstance(value, str):
        return interpolate(value, variables)
    if isinstance(value, list):
        return [interpolate_value(item, variables) for item in value]
    if isinstance(value, dict):
        return {
            interpolate(key, variables): interpolate_value(item, variables)
            for key, item in value.items()
        }
    return value


def match_value(expected: Any, actual: Any, variables: Mapping[str, str]) -> None:
    """Check ``actual`` against the authored matcher ``expected``.

    Raises :class:`MatchError` naming the path it failed at, so a mismatch deep
    in an envelope points at the field rather than at the whole body.
    """
    _check(expected, actual, variables, "$")


def _check(expected: Any, actual: Any, variables: Mapping[str, str], path: str) -> None:
    if isinstance(expected, dict):
        operator = _operator(expected, path)
        if operator is None:
            _check_subset(expected, actual, variables, path)
        else:
            _check_operator(*operator, actual, variables, path)
    elif isinstance(expected, list):
        _check_array(expected, actual, variables, path)
    elif isinstance(expected, str):
        _equal(interpolate(expected, variables), actual, path)
    else:
        _equal(expected, actual, path)


def _operator(expected: dict, path: str) -> tuple[str, Any] | None:
    """An operator object holds exactly one ``$`` key and nothing else.

    A mix of the two is a corpus bug that would otherwise pass silently.
    """
    dollars = [key for key in expected if key.startswith("$")]
    if not dollars:
        return None
    if len(dollars) != len(expected) or len(expected) != 1:
        raise MatchError(
            f"at {path}: an operator object holds exactly one $ key, "
            f"found {sorted(expected)}"
        )
    return dollars[0], expected[dollars[0]]


def _check_operator(
    operator: str, operand: Any, actual: Any, variables: Mapping[str, str], path: str
) -> None:
    if operator == "$exact":
        _equal(interpolate_value(operand, variables), actual, path)
    elif operator == "$min_length":
        wanted = _operand(operand, int, "$min_length takes a count", path)
        if not isinstance(actual, str):
            raise MatchError(f"at {path}: $min_length needs a string, got {_show(actual)}")
        if len(actual) < wanted:
            raise MatchError(f"at {path}: {actual!r} is shorter than {wanted} characters")
    elif operator == "$type":
        wanted = _operand(operand, str, "$type takes a type name", path)
        if wanted not in TYPES:
            raise MatchError(f"at {path}: unknown $type {wanted!r}")
        if not TYPES[wanted](actual):
            raise MatchError(f"at {path}: expected a {wanted}, got {_show(actual)}")
    elif operator == "$contains":
        _check_contains(operand, actual, variables, path)
    elif operator == "$prefix":
        wanted = interpolate(_operand(operand, str, "$prefix takes a string", path), variables)
        if not isinstance(actual, str):
            raise MatchError(f"at {path}: $prefix needs a string, got {_show(actual)}")
        if not actual.startswith(wanted):
            raise MatchError(f"at {path}: {actual!r} does not start with {wanted!r}")
    elif operator == "$gt":
        wanted = _operand(operand, (int, float), "$gt takes a number", path)
        if not _is_number(actual):
            raise MatchError(f"at {path}: $gt needs a number, got {_show(actual)}")
        if not actual > wanted:
            raise MatchError(f"at {path}: {actual} is not greater than {wanted}")
    else:
        raise MatchError(f"at {path}: unknown matcher {operator!r}")


def _operand(operand: Any, kind: type | tuple[type, ...], complaint: str, path: str) -> Any:
    """An operator's own argument, checked before it is used.

    A corpus that authors ``{"$gt": "x"}`` is a corpus bug, and must read as
    one rather than as whatever error using it happens to raise.
    """
    if not isinstance(operand, kind) or isinstance(operand, bool):
        raise MatchError(f"at {path}: {complaint}, got {_show(operand)}")
    return operand


def _check_contains(
    operand: Any, actual: Any, variables: Mapping[str, str], path: str
) -> None:
    if isinstance(actual, str):
        if not isinstance(operand, str):
            raise MatchError(f"at {path}: $contains on a string takes a string")
        needle = interpolate(operand, variables)
        if needle not in actual:
            raise MatchError(f"at {path}: {actual!r} does not contain {needle!r}")
        return
    if isinstance(actual, list):
        for item in actual:
            try:
                _check(operand, item, variables, path)
                return
            except MatchError:
                continue
        raise MatchError(
            f"at {path}: no element matches {_show(operand)}, got {_show(actual)}"
        )
    raise MatchError(f"at {path}: $contains needs a string or an array, got {_show(actual)}")


def _is_number(value: Any) -> bool:
    # `True` is an `int` in Python, and no JSON type calls it a number.
    return isinstance(value, (int, float)) and not isinstance(value, bool)


#: The type names ``$type`` takes, and what each one accepts.
TYPES = {
    "string": lambda value: isinstance(value, str),
    "number": _is_number,
    "integer": lambda value: isinstance(value, int) and not isinstance(value, bool),
    "boolean": lambda value: isinstance(value, bool),
    "array": lambda value: isinstance(value, list),
    "object": lambda value: isinstance(value, dict),
    "null": lambda value: value is None,
}


def _check_subset(
    expected: dict, actual: Any, variables: Mapping[str, str], path: str
) -> None:
    if not isinstance(actual, dict):
        raise MatchError(f"at {path}: expected an object, got {_show(actual)}")
    for key, want in expected.items():
        key = interpolate(key, variables)
        child = f"{path}.{key}"
        if key not in actual:
            raise MatchError(f"at {child}: missing")
        _check(want, actual[key], variables, child)


def _check_array(
    expected: list, actual: Any, variables: Mapping[str, str], path: str
) -> None:
    if not isinstance(actual, list):
        raise MatchError(f"at {path}: expected an array, got {_show(actual)}")
    if len(expected) != len(actual):
        raise MatchError(
            f"at {path}: expected {len(expected)} elements, got {len(actual)}"
        )
    for index, (want, got) in enumerate(zip(expected, actual)):
        _check(want, got, variables, f"{path}[{index}]")


def _equal(expected: Any, actual: Any, path: str) -> None:
    # `True == 1` in Python. A case that authors `true` means the boolean.
    if expected != actual or isinstance(expected, bool) is not isinstance(actual, bool):
        raise MatchError(f"at {path}: expected {_show(expected)}, got {_show(actual)}")


def _show(value: Any) -> str:
    try:
        return json.dumps(value)
    except TypeError:
        return repr(value)
