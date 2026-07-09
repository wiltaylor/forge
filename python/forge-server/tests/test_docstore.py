import json

from fastapi.testclient import TestClient

from forge_server import ForgeApp


def make_client(tmp_path):
    app = ForgeApp("docs")
    app.with_docstore(tmp_path / "data")
    return TestClient(app.fastapi), app


def test_invalid_names_400(tmp_path):
    client, _ = make_client(tmp_path)
    for bad in ["UPPER", "-leading", "_leading", "has.dot", "a" * 65, "sp ace", "%2e%2e"]:
        assert client.get(f"/api/data/{bad}").status_code == 400, bad
        assert client.put(f"/api/data/{bad}", json={}).status_code == 400, bad
        assert client.delete(f"/api/data/{bad}").status_code == 400, bad


def test_get_missing_404(tmp_path):
    client, _ = make_client(tmp_path)
    r = client.get("/api/data/nope")
    assert r.status_code == 404
    assert r.json() == {"ok": False, "error": "no document 'nope'"}


def test_put_get_roundtrip(tmp_path):
    client, _ = make_client(tmp_path)
    doc = {"colors": ["#ff0000", "#00ff00"], "n": 3, "nested": {"a": [1, 2]}}
    r = client.put("/api/data/state", json=doc)
    assert r.status_code == 200
    assert r.json() == {"ok": True}
    r = client.get("/api/data/state")
    assert r.status_code == 200
    assert r.json() == {"ok": True, "data": doc}
    # one file per doc, no leftover tmp file
    assert (tmp_path / "data" / "state.json").exists()
    assert not (tmp_path / "data" / "state.json.tmp").exists()
    assert json.loads((tmp_path / "data" / "state.json").read_text()) == doc


def test_put_replaces(tmp_path):
    client, _ = make_client(tmp_path)
    client.put("/api/data/state", json={"v": 1})
    client.put("/api/data/state", json={"v": 2})
    assert client.get("/api/data/state").json()["data"] == {"v": 2}


def test_put_non_object_json(tmp_path):
    client, _ = make_client(tmp_path)
    assert client.put("/api/data/scalar", json=[1, 2, 3]).status_code == 200
    assert client.get("/api/data/scalar").json()["data"] == [1, 2, 3]


def test_put_invalid_json_400(tmp_path):
    client, _ = make_client(tmp_path)
    r = client.put(
        "/api/data/state", content=b"{not json", headers={"content-type": "application/json"}
    )
    assert r.status_code == 400
    assert "JSON" in r.json()["error"]


def test_delete_idempotent(tmp_path):
    client, _ = make_client(tmp_path)
    client.put("/api/data/gone", json={"x": 1})
    assert client.delete("/api/data/gone").json() == {"ok": True}
    assert client.delete("/api/data/gone").json() == {"ok": True}  # second delete OK
    assert client.get("/api/data/gone").status_code == 404


def test_list(tmp_path):
    client, _ = make_client(tmp_path)
    assert client.get("/api/data").json() == {"ok": True, "data": []}
    client.put("/api/data/beta", json={"b": 1})
    client.put("/api/data/alpha", json={"a": 1})
    docs = client.get("/api/data").json()["data"]
    assert [d["name"] for d in docs] == ["alpha", "beta"]  # sorted
    for d in docs:
        assert d["bytes"] > 0
        assert isinstance(d["modified"], float)  # unix seconds
