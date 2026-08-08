"""The document store called directly — no server, only a temporary directory.

The rules live in :mod:`forge_server.core.docstore`. The client tests at the
bottom cover only what the route adds: request-body parsing, URL decoding,
and one envelope per error kind. The wire itself is the corpus's business
(test_corpus.py).
"""

import json

import pytest

from forge_server.core.docstore import DocStore
from forge_server.core.error import BadRequest, Internal, NotFound


@pytest.mark.parametrize("op", ["read", "write", "delete"])
@pytest.mark.parametrize(
    "bad",
    ["UPPER", "-leading", "_leading", "has.dot", "a" * 65, "sp ace", "..", "../secret"],
)
def test_name_rule_refuses_read_write_and_delete(tmp_path, bad, op):
    store = DocStore(tmp_path)
    call = {
        "read": store.read,
        "write": lambda name: store.write(name, {}),
        "delete": store.delete,
    }[op]
    with pytest.raises(BadRequest) as e:
        call(bad)
    assert e.value.status == 400


def test_path_joins_valid_names(tmp_path):
    assert DocStore(tmp_path).path("notes") == tmp_path / "notes.json"


def test_missing_doc_is_a_404(tmp_path):
    with pytest.raises(NotFound) as e:
        DocStore(tmp_path).read("nope")
    assert (e.value.status, e.value.message) == (404, "no document 'nope'")


def test_write_read_roundtrip(tmp_path):
    store = DocStore(tmp_path / "data")  # write() creates the directory
    doc = {"colors": ["#ff0000", "#00ff00"], "n": 3, "nested": {"a": [1, 2]}}
    store.write("state", doc)
    assert store.read("state") == doc
    # one file per doc, no leftover tmp file
    assert (tmp_path / "data" / "state.json").exists()
    assert not (tmp_path / "data" / "state.json.tmp").exists()
    assert json.loads((tmp_path / "data" / "state.json").read_text()) == doc


def test_write_replaces(tmp_path):
    store = DocStore(tmp_path)
    store.write("state", {"v": 1})
    store.write("state", {"v": 2})
    assert store.read("state") == {"v": 2}


def test_write_non_object_value(tmp_path):
    store = DocStore(tmp_path)
    store.write("scalar", [1, 2, 3])
    assert store.read("scalar") == [1, 2, 3]


def test_delete_idempotent(tmp_path):
    store = DocStore(tmp_path)
    store.write("gone", {"x": 1})
    store.delete("gone")
    store.delete("gone")  # second delete OK
    with pytest.raises(NotFound):
        store.read("gone")


def test_list(tmp_path):
    store = DocStore(tmp_path / "data")
    assert store.list() == []  # absent directory is an empty store
    store.write("beta", {"b": 1})
    store.write("alpha", {"a": 1})
    docs = store.list()
    assert [d["name"] for d in docs] == ["alpha", "beta"]  # sorted
    for d in docs:
        assert d["bytes"] > 0
        assert isinstance(d["modified"], float)  # unix seconds


def test_a_corrupt_doc_file_is_a_500(tmp_path):
    (tmp_path / "notes.json").write_text("{ not json")
    with pytest.raises(Internal) as e:
        DocStore(tmp_path).read("notes")
    assert e.value.status == 500


# -- the route: what HTTP adds ---------------------------------------------


def make_client(tmp_path):
    from fastapi.testclient import TestClient

    from forge_server import ForgeApp

    app = ForgeApp("docs")
    app.with_docstore(tmp_path / "data")
    return TestClient(app.fastapi)


def test_route_decodes_the_name_before_the_rule(tmp_path):
    # %2e%2e reaches the rule as ".." — decoding is the wire's business,
    # and the refusal must come out as the 400 envelope.
    r = make_client(tmp_path).get("/api/data/%2e%2e")
    assert r.status_code == 400
    assert r.json()["ok"] is False


def test_route_maps_not_found_to_the_envelope(tmp_path):
    r = make_client(tmp_path).get("/api/data/nope")
    assert r.status_code == 404
    assert r.json() == {"ok": False, "error": "no document 'nope'"}


def test_put_invalid_json_body_400(tmp_path):
    # Body parsing lives in the route, not the store.
    r = make_client(tmp_path).put(
        "/api/data/state", content=b"{not json", headers={"content-type": "application/json"}
    )
    assert r.status_code == 400
    assert "JSON" in r.json()["error"]
