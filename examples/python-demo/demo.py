#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["forge-server"]
#
# [tool.uv.sources]
# forge-server = { path = "../../python/forge-server", editable = true }
# # From another repo, use the git form instead:
# # forge-server = { git = "https://github.com/wiltaylor/forge", subdirectory = "python/forge-server" }
# ///
"""Forge python-demo — the same gallery app on the Python backend.

Run: uv run demo.py   (from examples/python-demo; reads .env here)
"""
import asyncio
import contextlib
import os

from forge_server import ForgeApp

app = ForgeApp("python-demo")
if os.environ.get("FORGE_JWT_SECRET"):
    app.auth_from_env()
app.with_docstore()          # FORGE_DATA_DIR, default ./data
app.with_events()
app.with_components()        # FORGE_COMPONENTS_DIR
app.serve_frontend("../../apps/gallery/dist", spa=True)


@app.action("echo")
def echo(payload):
    return payload


@app.action("publish")
def publish(payload, ctx):
    topic = str(payload.get("topic", "misc"))
    ctx.events.publish(topic, payload.get("data"))
    return {"published": True, "topic": topic}


@app.fastapi.on_event("startup")
async def start_ticker():
    async def tick():
        n = 0
        while True:
            await asyncio.sleep(2)
            n += 1
            with contextlib.suppress(Exception):
                app.events.publish("ticks", {"n": n, "source": "python-demo"})

    asyncio.create_task(tick())


if __name__ == "__main__":
    app.serve()
