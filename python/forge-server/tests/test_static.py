import pytest
from fastapi.testclient import TestClient

from forge_server import ForgeApp

INDEX_HTML = "<!doctype html><title>forge test</title><div id='app'></div>"


@pytest.fixture
def dist(tmp_path):
    d = tmp_path / "dist"
    (d / "assets").mkdir(parents=True)
    (d / "index.html").write_text(INDEX_HTML)
    (d / "assets" / "app.js").write_text("console.log('hi')")
    (d / "favicon.ico").write_bytes(b"\x00icon")
    return d


def make_client(tmp_path, dist, spa=True):
    app = ForgeApp("static")
    app.with_docstore(tmp_path / "data")
    app.serve_frontend(dist, spa=spa)
    return TestClient(app.fastapi), app


def test_index_served_at_root(tmp_path, dist):
    client, _ = make_client(tmp_path, dist)
    r = client.get("/")
    assert r.status_code == 200
    assert r.text == INDEX_HTML


def test_spa_fallback_for_unknown_paths(tmp_path, dist):
    client, _ = make_client(tmp_path, dist)
    for path in ["/dashboard", "/settings/profile", "/deep/nested/route"]:
        r = client.get(path)
        assert r.status_code == 200, path
        assert r.text == INDEX_HTML, path


def test_real_files_served(tmp_path, dist):
    client, _ = make_client(tmp_path, dist)
    assert client.get("/assets/app.js").text == "console.log('hi')"
    assert client.get("/favicon.ico").content == b"\x00icon"


def test_api_misses_stay_json_404(tmp_path, dist):
    client, _ = make_client(tmp_path, dist)
    r = client.get("/api/definitely/not/a/route")
    assert r.status_code == 404
    body = r.json()
    assert body["ok"] is False
    assert "error" in body


def test_api_routes_still_work(tmp_path, dist):
    client, _ = make_client(tmp_path, dist)
    assert client.get("/api/health").status_code == 200
    client.put("/api/data/state", json={"x": 1})
    assert client.get("/api/data/state").json()["data"] == {"x": 1}


def test_routes_registered_after_serve_frontend_win(tmp_path, dist):
    app = ForgeApp("static")
    app.serve_frontend(dist)

    @app.get("/api/custom")
    async def custom():
        return {"ok": True, "data": "custom"}

    client = TestClient(app.fastapi)
    assert client.get("/api/custom").json()["data"] == "custom"
    assert client.get("/anything").text == INDEX_HTML  # catch-all still last


def test_no_spa_fallback_when_disabled(tmp_path, dist):
    client, _ = make_client(tmp_path, dist, spa=False)
    assert client.get("/").status_code == 200  # index still served at root
    assert client.get("/assets/app.js").status_code == 200
    assert client.get("/unknown-route").status_code == 404


def test_traversal_blocked(tmp_path, dist):
    (tmp_path / "secret.txt").write_text("nope")
    client, _ = make_client(tmp_path, dist)
    r = client.get("/../secret.txt")
    # either normalized to a miss (SPA fallback) or rejected — never the file
    assert "nope" not in r.text
