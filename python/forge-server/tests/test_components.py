"""Component federation. The rule and the manifest are called directly — no
server, no test client; only a temporary directory."""

from pathlib import Path

import pytest

from forge_server.core.components import Components, valid_component_file
from forge_server.core.error import BadRequest, ForgeError


@pytest.mark.parametrize(
    "name",
    ["widget.js", "Widget-1.2.3.mjs", "styles.css", "bundle.js.map", "a" * 125 + ".js"],
)
def test_filename_accepted(name):
    assert valid_component_file(name)


@pytest.mark.parametrize(
    "name",
    [
        "evil.sh",
        ".hidden.js",
        "no-ext",
        "a/../b.js",
        "a..b.js",
        "",
        "a" * 126 + ".js",
        "widget.js\n",
    ],
)
def test_filename_rejected(name):
    assert not valid_component_file(name)


def test_file_path_rejects_traversal(tmp_path):
    with pytest.raises(BadRequest) as e:
        Components(tmp_path).file_path("../secret.js")
    assert e.value.status == 400


def test_file_path_joins_valid_names(tmp_path):
    assert Components(tmp_path).file_path("widget.js") == tmp_path / "widget.js"


def test_absent_manifest_is_an_empty_catalogue(tmp_path):
    assert Components(tmp_path).manifest("demo") == {"app": "demo", "components": []}


def test_object_manifest_gets_the_app_name_injected(tmp_path):
    (tmp_path / "manifest.json").write_text(
        '{"app": "stale", "components": [{"name": "Widget"}]}'
    )
    assert Components(tmp_path).manifest("demo") == {
        "app": "demo",
        "components": [{"name": "Widget"}],
    }


def test_array_manifest_is_the_components_list(tmp_path):
    (tmp_path / "manifest.json").write_text('[{"name": "Widget"}]')
    assert Components(tmp_path).manifest("demo") == {
        "app": "demo",
        "components": [{"name": "Widget"}],
    }


def test_corrupt_manifest_is_a_500(tmp_path):
    (tmp_path / "manifest.json").write_text("{ not json")
    with pytest.raises(ForgeError) as e:
        Components(tmp_path).manifest("demo")
    assert e.value.status == 500


def test_manifest_that_is_neither_object_nor_array_is_a_500(tmp_path):
    (tmp_path / "manifest.json").write_text('"hello"')
    with pytest.raises(ForgeError) as e:
        Components(tmp_path).manifest("demo")
    assert e.value.status == 500


def make_client(tmp_path):
    from fastapi.testclient import TestClient

    from forge_server import ForgeApp

    app = ForgeApp("comps")
    app.with_components(tmp_path)
    return TestClient(app.fastapi)


def test_endpoint_serves_the_manifest(tmp_path):
    (tmp_path / "manifest.json").write_text('{"components": [{"name": "Widget"}]}')
    r = make_client(tmp_path).get("/api/components")
    assert r.status_code == 200
    assert r.json() == {
        "ok": True,
        "data": {"app": "comps", "components": [{"name": "Widget"}]},
    }


def test_endpoint_without_a_manifest_is_an_empty_catalogue(tmp_path):
    r = make_client(tmp_path).get("/api/components")
    assert r.status_code == 200
    assert r.json() == {"ok": True, "data": {"app": "comps", "components": []}}


def test_endpoint_serves_a_bundle(tmp_path):
    (tmp_path / "widget.js").write_text("export const widget = 1;\n")
    r = make_client(tmp_path).get("/api/components/widget.js")
    assert r.status_code == 200
    assert "export const widget" in r.text


@pytest.mark.parametrize(
    "bad", ["%2E%2E%2Fsecret.js", "..secret.js", "evil.sh", ".hidden.js", "a/b.js"]
)
def test_endpoint_rejects_a_name_the_rule_refuses(tmp_path, bad):
    # The route takes the whole path tail, so a decoded separator reaches the
    # rule rather than missing the route. See the comment in components.py.
    r = make_client(tmp_path).get(f"/api/components/{bad}")
    assert r.status_code == 400, r.text
    assert r.json()["ok"] is False


def test_endpoint_missing_bundle_is_404(tmp_path):
    r = make_client(tmp_path).get("/api/components/nope.js")
    assert r.status_code == 404
    assert r.json() == {"ok": False, "error": "no component file 'nope.js'"}


def test_core_module_holds_no_web_framework_import():
    """The rule stays callable from a non-HTTP host. The package ``__init__``
    still pulls in the framework — that is issue #43's acceptance criterion."""
    from forge_server.core import components

    source = Path(components.__file__).read_text()
    assert "fastapi" not in source
    assert "starlette" not in source
