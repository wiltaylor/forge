"""Tests for the corpus reader and the matcher the driver runs on.

These guard the driver's instruments. A matcher that says yes too easily, or a
reader that skips a field it does not know, makes every case in
``test_corpus.py`` weaker without failing anything.
"""

import json

import pytest

from contract import PYTHON_HTTP, Corpus, CorpusError, MatchError, interpolate_value, match_value
from contract.corpus import CORPUS_PATH

VARS = {"user": "admin"}

AUTHORED = CORPUS_PATH.read_text()


def ok(expected, actual):
    match_value(expected, actual, VARS)


def fails(expected, actual) -> str:
    with pytest.raises(MatchError) as e:
        match_value(expected, actual, VARS)
    return str(e.value)


# -- the authored corpus ---------------------------------------------------


def test_the_authored_corpus_is_valid():
    corpus = Corpus.load()
    assert corpus.contract_version == "1.0"
    assert corpus.cases_for(PYTHON_HTTP)


def test_a_case_that_ignores_a_transport_is_rejected():
    """The rule that keeps gaps visible. Drop a transport from a case and the
    corpus must refuse to load."""
    corpus = json.loads(AUTHORED)
    corpus["cases"][0]["applies"] = ["rust-http"]
    corpus["cases"][0]["inapplicable"] = {}
    with pytest.raises(CorpusError, match="says nothing about transport 'python-http'"):
        Corpus.parse(json.dumps(corpus))


def test_an_excuse_needs_a_reason():
    corpus = json.loads(AUTHORED)
    case = next(c for c in corpus["cases"] if c["inapplicable"])
    for transport in case["inapplicable"]:
        case["inapplicable"][transport] = "  "
    with pytest.raises(CorpusError, match="with no reason"):
        Corpus.parse(json.dumps(corpus))


def test_a_transport_cannot_both_apply_and_be_excused():
    corpus = json.loads(AUTHORED)
    corpus["cases"][0]["inapplicable"] = {PYTHON_HTTP: "cannot"}
    with pytest.raises(CorpusError, match="both applies to and excuses"):
        Corpus.parse(json.dumps(corpus))


def test_a_stream_cannot_be_asked_for_a_body():
    """A body authored on the stream-opening step would be dropped by the
    driver, so the corpus refuses it rather than looking like coverage."""
    corpus = json.loads(AUTHORED)
    case = next(c for c in corpus["cases"] if c.get("kind") == "sse")
    case["steps"][0]["expect"]["body"] = {"ok": True}
    with pytest.raises(CorpusError, match="expects a body from the request"):
        Corpus.parse(json.dumps(corpus))


def test_duplicate_case_ids_are_rejected():
    corpus = json.loads(AUTHORED)
    corpus["cases"].append(corpus["cases"][0])
    with pytest.raises(CorpusError, match="duplicate case id"):
        Corpus.parse(json.dumps(corpus))


def test_an_unknown_field_is_not_read_past():
    """A field the reader does not know is a typo, and a typo in an
    expectation is an assertion that never runs."""
    corpus = json.loads(AUTHORED)
    corpus["cases"][0]["steps"][0]["expect"]["bodyy"] = {"ok": True}
    with pytest.raises(CorpusError, match="unknown field"):
        Corpus.parse(json.dumps(corpus))


def test_a_step_the_reader_does_not_know_is_rejected():
    corpus = json.loads(AUTHORED)
    corpus["cases"][0]["steps"].append({"await_nothing": {}})
    with pytest.raises(CorpusError, match="not a step"):
        Corpus.parse(json.dumps(corpus))


# -- the matcher -----------------------------------------------------------


def test_objects_match_by_subset():
    ok({"ok": True}, {"ok": True, "data": 1})
    assert "$.ok: missing" in fails({"ok": True}, {"data": 1})


def test_exact_rejects_extra_keys():
    ok({"$exact": {"n": 1}}, {"n": 1})
    assert "expected" in fails({"$exact": {"n": 1}}, {"n": 1, "x": 2})


def test_arrays_match_element_wise_and_by_length():
    ok([{"a": 1}], [{"a": 1, "b": 2}])
    assert "expected 1 elements, got 2" in fails([1], [1, 2])


def test_contains_reads_strings_and_arrays():
    ok({"$contains": "echo"}, ["echo", "publish"])
    ok({"$contains": "echo"}, "no action echo here")
    ok({"$contains": {"name": "b"}}, [{"name": "a"}, {"name": "b"}])
    assert "no element" in fails({"$contains": "gone"}, ["echo"])


def test_type_and_number_matchers():
    ok({"$type": "integer"}, 3)
    ok({"$gt": 0}, 1.5)
    assert "expected a integer" in fails({"$type": "integer"}, 1.5)
    assert "not greater" in fails({"$gt": 0}, 0)


def test_a_boolean_is_not_the_number_one():
    """Python says ``True == 1``; the contract does not."""
    assert "expected true" in fails(True, 1)
    assert "expected a integer" in fails({"$type": "integer"}, True)


def test_min_length_rejects_the_empty_string():
    """``$type: "string"`` would accept ``""``, which is what a token must
    never be."""
    ok({"$min_length": 1}, "a")
    assert "shorter than 1" in fails({"$min_length": 1}, "")
    assert "needs a string" in fails({"$min_length": 1}, 7)


def test_strings_interpolate_before_comparing():
    ok("${user}", "admin")
    assert 'expected "admin"' in fails("${user}", "ops")
    assert "unknown variable" in fails("${nope}", "x")


def test_a_mixed_operator_object_is_a_corpus_bug():
    assert "exactly one $ key" in fails({"$type": "object", "n": 1}, {"n": 1})


def test_interpolation_reaches_keys_and_nested_values():
    assert interpolate_value({"${user}": ["${user}", 1]}, VARS) == {"admin": ["admin", 1]}


def test_the_path_in_the_error_points_at_the_field():
    error = fails({"data": {"sub": "${user}"}}, {"data": {"sub": "ops"}})
    assert error.startswith("at $.data.sub:")
