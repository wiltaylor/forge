"""The seam itself: the core is framework-free and its verdicts carry statuses.

What each rule *does* is tested next to the rule — test_docstore.py,
test_actions.py, test_events.py and test_components.py call their subjects
directly. This file only proves the seam: no web-framework import, domain
errors carrying the status the routing layer maps, and a core that still
works with the framework refused at import time.
"""

import subprocess
import sys
import textwrap
from pathlib import Path

import pytest

from forge_server.core import actions as core_actions
from forge_server.core import components as core_components
from forge_server.core import docstore as core_docstore
from forge_server.core import error as core_error
from forge_server.core import events as core_events
from forge_server.core.error import BadRequest, ForgeError, Internal, NotFound

CORE_MODULES = [
    core_actions,
    core_components,
    core_docstore,
    core_error,
    core_events,
]


@pytest.mark.parametrize("module", CORE_MODULES, ids=lambda m: m.__name__.split(".")[-1])
def test_core_module_holds_no_web_framework_import(module):
    source = Path(module.__file__).read_text()
    for framework in ("fastapi", "starlette", "uvicorn"):
        assert framework not in source, f"{module.__name__} names {framework}"


def test_domain_errors_carry_the_status_the_routing_layer_maps():
    kinds = (BadRequest, NotFound, Internal)
    assert [kind("x").status for kind in kinds] == [400, 404, 500]
    assert all(issubclass(kind, ForgeError) for kind in kinds)
    assert ForgeError("x").message == "x"


# The core is worth nothing as a seam if it only works where the framework
# happens to be installed, so this runs in a fresh interpreter with every
# framework module refused at import time, and calls the rules there.
_WITHOUT_THE_FRAMEWORK = textwrap.dedent(
    '''
    import asyncio, sys, tempfile

    BLOCKED = ("fastapi", "starlette", "sse_starlette", "uvicorn")

    class Blocker:
        def find_spec(self, name, path=None, target=None):
            if name.split(".")[0] in BLOCKED:
                raise ImportError(f"{name} is absent for this test")
            return None

    sys.meta_path.insert(0, Blocker())

    import forge_server  # the package __init__ must not drag the framework in
    from forge_server.core import (
        ActionContext, ActionRegistry, BadRequest, Components, DocStore,
        EventBus, ForgeError, NotFound,
    )

    with tempfile.TemporaryDirectory() as tmp:
        store = DocStore(tmp)
        store.write("notes", {"a": 1})
        assert store.read("notes") == {"a": 1}
        assert [d["name"] for d in store.list()] == ["notes"]
        store.delete("notes")

        try:
            store.path("UPPER")
        except BadRequest as e:
            assert e.status == 400
        else:
            raise AssertionError("the doc-name rule did not refuse")

        try:
            store.read("nope")
        except NotFound as e:
            assert e.status == 404
        else:
            raise AssertionError("a missing doc did not 404")

        assert Components(tmp).manifest("demo") == {"app": "demo", "components": []}

    registry = ActionRegistry()
    registry.register("echo", lambda payload: payload)
    ctx = ActionContext(claims={}, app=None, events=None)
    assert asyncio.run(registry.dispatch("echo", {"n": 1}, ctx)) == {"n": 1}

    async def fanout():
        bus = EventBus()
        sub = bus.subscribe()
        bus.publish("tick", {"n": 1})
        return await sub.queue.get()

    assert asyncio.run(fanout()) == ("tick", {"n": 1})

    assert not [m for m in sys.modules if m.split(".")[0] in BLOCKED]
    print("ok")
    '''
)


def test_core_is_importable_and_callable_with_the_framework_absent():
    proc = subprocess.run(
        [sys.executable, "-c", _WITHOUT_THE_FRAMEWORK],
        capture_output=True,
        text=True,
    )
    assert proc.returncode == 0, proc.stderr
    assert proc.stdout.strip().endswith("ok")


def test_importing_the_app_still_works():
    """The lazy package ``__init__`` must not break the documented import."""
    from forge_server import ForgeApp

    assert ForgeApp("x").name == "x"
    import forge_server

    assert forge_server.__version__
    with pytest.raises(AttributeError):
        forge_server.NoSuchThing
