#!/usr/bin/env -S uv run
# /// script
# requires-python = ">=3.11"
# dependencies = ["forge-server", "httpx>=0.28", "pytest>=9", "websockets>=16"]
#
# [tool.uv.sources]
# forge-server = { path = "../python/forge-server", editable = true }
# ///
"""Run the black-box parity suite against a server this script starts itself.

`just parity-test` runs the suite against a server that is already up — either
backend, which is what the suite is for. This script is what `just test` uses:
it starts the Python backend in the demo configuration the suite documents, on
its own port and in a throwaway directory, runs `examples/parity` against it,
and stops it again.

Run via `just parity-test-local`.
"""
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PORT = int(os.environ.get("FORGE_PARITY_PORT", "18765"))
BASE = f"http://127.0.0.1:{PORT}"
SECRET = "parity-harness-secret-32-characters!!"


def serve() -> None:
    """The demo configuration: auth, doc store, events, `echo` and `publish`."""
    from forge_server import ForgeApp

    app = ForgeApp("parity-harness")
    app.auth_from_env()
    app.with_docstore()
    app.with_events()
    app.with_components()
    app.serve_frontend("frontend", spa=True)

    @app.action("echo")
    def echo(payload):
        return payload

    @app.action("publish")
    def publish(payload, ctx):
        topic = str(payload.get("topic", "misc"))
        ctx.events.publish(topic, payload.get("data"))
        return {"published": True, "topic": topic}

    app.serve()


def wait_for_health(server: subprocess.Popen, log: Path) -> None:
    import httpx

    for _ in range(300):
        if server.poll() is not None:
            break
        try:
            if httpx.get(f"{BASE}/api/health", timeout=1).status_code == 200:
                return
        except httpx.HTTPError:
            time.sleep(0.1)
    print(f"the parity server on :{PORT} did not come up; its log:\n{log.read_text()}")
    sys.exit(1)


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="forge-parity-") as tmp:
        root = Path(tmp)
        # The suite asks for a client route and expects the SPA fallback, so the
        # harness needs a frontend — any frontend.
        (root / "frontend").mkdir()
        (root / "frontend/index.html").write_text("<!doctype html>\n<title>parity harness</title>\n")

        env = os.environ | {
            "FORGE_HOST": "127.0.0.1",
            "FORGE_PORT": str(PORT),
            "FORGE_JWT_SECRET": SECRET,
            "FORGE_AUTH_USERS": "admin:admin",
        }
        log = root / "server.log"
        with log.open("w") as sink:
            # Started in the throwaway directory: its doc store, components dir
            # and .env resolve there rather than in the repo.
            server = subprocess.Popen(
                [sys.executable, __file__, "--serve"], env=env, cwd=root, stdout=sink, stderr=sink
            )
            try:
                wait_for_health(server, log)
                return subprocess.run(
                    [sys.executable, "-m", "pytest", str(REPO / "examples/parity"), "-q"],
                    env=env | {"FORGE_TEST_BASE_URL": BASE},
                    cwd=REPO,
                ).returncode
            finally:
                server.terminate()
                try:
                    server.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    server.kill()


if __name__ == "__main__":
    if "--serve" in sys.argv:
        serve()
    else:
        sys.exit(main())
