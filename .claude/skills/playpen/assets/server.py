#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["fastapi>=0.115", "uvicorn>=0.30"]
# ///
"""Playpen server — JSON document store, custom actions, static dist/ serving.

Extend this file per playground:
  - add custom actions to the ACTIONS dict (see the `echo` example)
  - add bespoke routes only when an action doesn't fit (e.g. repo file export)
The generic /api/data document store usually needs no changes.
"""

import argparse
import json
import re
import time
from datetime import datetime, timezone
from pathlib import Path

from fastapi import FastAPI, HTTPException, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles

SERVER_DIR = Path(__file__).parent.resolve()
DATA_DIR = SERVER_DIR / "data"
DIST_DIR = SERVER_DIR.parent / "www" / "dist"
NAME_RE = re.compile(r"^[a-z0-9][a-z0-9_-]{0,63}$")
START = time.time()

app = FastAPI(title="playpen")

# Dev traffic normally arrives through the Vite proxy (same-origin); CORS is
# belt-and-braces for direct calls from the :5173 origin.
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173", "http://127.0.0.1:5173"],
    allow_methods=["*"],
    allow_headers=["*"],
)


def doc_path(name: str) -> Path:
    if not NAME_RE.match(name):
        raise HTTPException(400, f"invalid document name: {name!r} (must match {NAME_RE.pattern})")
    return DATA_DIR / f"{name}.json"


@app.get("/api/health")
def health():
    return {
        "ok": True,
        "uptime_s": round(time.time() - START, 1),
        "dist_built": DIST_DIR.exists(),
        "actions": sorted(ACTIONS),
    }


@app.get("/api/data")
def list_docs():
    docs = []
    if DATA_DIR.exists():
        for p in sorted(DATA_DIR.glob("*.json")):
            docs.append({
                "name": p.stem,
                "bytes": p.stat().st_size,
                "modified": datetime.fromtimestamp(p.stat().st_mtime, timezone.utc).isoformat(),
            })
    return {"ok": True, "data": docs}


@app.get("/api/data/{name}")
def get_doc(name: str):
    p = doc_path(name)
    if not p.exists():
        raise HTTPException(404, f"no document {name!r}")
    return {"ok": True, "data": json.loads(p.read_text())}


@app.put("/api/data/{name}")
async def put_doc(name: str, request: Request):
    p = doc_path(name)
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    try:
        body = json.loads(await request.body() or b"null")
    except json.JSONDecodeError as e:
        raise HTTPException(400, f"body is not valid JSON: {e}")
    tmp = p.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(body, indent=2))
    tmp.replace(p)  # atomic on POSIX
    return {"ok": True}


@app.delete("/api/data/{name}")
def delete_doc(name: str):
    doc_path(name).unlink(missing_ok=True)
    return {"ok": True}


# ---- custom actions ---------------------------------------------------------
# An action is a function taking the JSON payload (dict) and returning any
# JSON-able value. Register it in ACTIONS; the UI calls it via
# POST /api/actions/<name> and the CLI via `playpen call POST /api/actions/<name>`.

def echo(payload: dict):
    return payload


ACTIONS = {
    "echo": echo,
}


@app.post("/api/actions/{name}")
async def run_action(name: str, request: Request):
    if name not in ACTIONS:
        raise HTTPException(404, f"unknown action {name!r} (have: {sorted(ACTIONS)})")
    raw = await request.body()
    try:
        payload = json.loads(raw) if raw else {}
    except json.JSONDecodeError as e:
        raise HTTPException(400, f"body is not valid JSON: {e}")
    return {"ok": True, "data": ACTIONS[name](payload)}


# ---- static -----------------------------------------------------------------
# Mounted last so /api routes win. Serves the Vite build in --build mode; in
# dev mode the UI lives on the Vite dev server and this is just a hint page.
if DIST_DIR.exists():
    app.mount("/", StaticFiles(directory=DIST_DIR, html=True))
else:
    @app.get("/", response_class=HTMLResponse)
    def hint():
        return (
            "<pre>playpen: no frontend build yet.\n\n"
            "Dev mode:   uv run .playpen/playpen.py start          (UI on :5173)\n"
            "Build mode: uv run .playpen/playpen.py start --build  (UI served here)\n"
            "</pre>"
        )


if __name__ == "__main__":
    import uvicorn

    parser = argparse.ArgumentParser(description="playpen FastAPI server")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--host", default="127.0.0.1")
    args = parser.parse_args()
    uvicorn.run(app, host=args.host, port=args.port)
