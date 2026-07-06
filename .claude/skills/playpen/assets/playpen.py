#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""playpen CLI — lifecycle, data access, and API calls for the .playpen app.

Run from anywhere: uv run .playpen/playpen.py <command>

Commands:
  start [--build] [--port N] [--vite-port N]  start FastAPI (+ Vite dev unless --build)
  stop                                        stop all playpen processes
  restart [--build] [--port N] [--vite-port N]
  status                                      JSON: pids, liveness, health
  logs [server|vite] [-n N]                   tail a daemon log (default: server, 40)
  build                                       npm install (if needed) + vite build
  data list                                   list persisted documents
  data get NAME                               print a document as JSON
  data set NAME (--json S | --file F | -)     write a document (- reads stdin)
  data delete NAME                            delete a document
  call METHOD PATH [--json S]                 arbitrary API call, prints JSON

Data commands use the HTTP API when the server is up and fall back to direct
file access in server/data/ when it is down.
"""

import argparse
import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).parent.resolve()  # .playpen/
RUN = ROOT / "run"
WWW = ROOT / "www"
SERVER = ROOT / "server"
DATA = SERVER / "data"

DEFAULT_PORT = int(os.environ.get("PLAYPEN_PORT", 8765))
DEFAULT_VITE_PORT = int(os.environ.get("PLAYPEN_VITE_PORT", 5173))


def die(msg: str, code: int = 1):
    print(f"playpen: {msg}", file=sys.stderr)
    sys.exit(code)


# ---- process helpers --------------------------------------------------------

def pidfile(name: str) -> Path:
    return RUN / f"{name}.pid"


def read_pid(name: str):
    try:
        return int(pidfile(name).read_text().strip())
    except (FileNotFoundError, ValueError):
        return None


def alive(pid) -> bool:
    if pid is None:
        return False
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def clean_stale(name: str):
    if read_pid(name) is not None and not alive(read_pid(name)):
        pidfile(name).unlink(missing_ok=True)


def port_busy(port: int) -> bool:
    # Probe both address families — Vite/Node may bind ::1 for "localhost".
    for family, addr in ((socket.AF_INET, "127.0.0.1"), (socket.AF_INET6, "::1")):
        try:
            with socket.socket(family) as s:
                if s.connect_ex((addr, port)) == 0:
                    return True
        except OSError:
            continue
    return False


def spawn(name: str, cmd: list, cwd: Path, extra_env: dict | None = None) -> int:
    RUN.mkdir(exist_ok=True)
    log = open(RUN / f"{name}.log", "ab")
    proc = subprocess.Popen(
        cmd,
        cwd=cwd,
        stdout=log,
        stderr=log,
        start_new_session=True,  # own process group so stop can killpg children
        env={**os.environ, **(extra_env or {})},
    )
    pidfile(name).write_text(str(proc.pid))
    return proc.pid


def wait_for(pred, timeout: float, what: str, log_name: str):
    start = time.time()
    while time.time() - start < timeout:
        if pred():
            return
        time.sleep(0.3)
    print(f"--- last lines of {log_name} log ---", file=sys.stderr)
    tail_log(log_name, 20, file=sys.stderr)
    die(f"timed out after {timeout:.0f}s waiting for {what}")


def tail_log(name: str, n: int, file=sys.stdout):
    path = RUN / f"{name}.log"
    if not path.exists():
        print(f"(no {name} log at {path})", file=file)
        return
    lines = path.read_text(errors="replace").splitlines()
    for line in lines[-n:]:
        print(line, file=file)


# ---- HTTP helpers -----------------------------------------------------------

def api(method: str, path: str, body=None, port: int = DEFAULT_PORT, timeout: float = 10):
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}{path}",
        method=method,
        data=json.dumps(body).encode() if body is not None else None,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read() or b"{}")


def server_up(port: int = DEFAULT_PORT) -> bool:
    try:
        return api("GET", "/api/health", port=port, timeout=2).get("ok", False)
    except (urllib.error.URLError, OSError, json.JSONDecodeError):
        return False


# ---- node helpers -----------------------------------------------------------

def ensure_node():
    if not shutil.which("npm"):
        die("npm not found — playpen needs Node 20+ (https://nodejs.org)")


def ensure_node_modules():
    if not (WWW / "node_modules").exists():
        print("Installing npm dependencies (first run, may take 30-90s)...")
        result = subprocess.run(["npm", "install"], cwd=WWW)
        if result.returncode != 0:
            die("npm install failed — see output above")


# ---- commands ---------------------------------------------------------------

def cmd_start(args):
    for name in ("server", "vite"):
        clean_stale(name)
    if alive(read_pid("server")):
        die("already running — check `playpen status`, or `playpen stop` first")
    if port_busy(args.port):
        die(f"port {args.port} is in use — free it or pass --port")

    if args.build:
        cmd_build(args)

    spawn("server", ["uv", "run", "server.py", "--port", str(args.port)], cwd=SERVER)
    wait_for(lambda: server_up(args.port), 60, "FastAPI server", "server")

    if args.build:
        print(f"UI: http://localhost:{args.port}  (built frontend, single process)")
        return

    ensure_node()
    ensure_node_modules()
    if port_busy(args.vite_port):
        die(f"port {args.vite_port} is in use — free it or pass --vite-port")
    spawn(
        "vite",
        ["npm", "run", "dev", "--", "--port", str(args.vite_port), "--strictPort"],
        cwd=WWW,
        extra_env={"PLAYPEN_API": f"http://127.0.0.1:{args.port}"},
    )
    wait_for(lambda: port_busy(args.vite_port), 60, "Vite dev server", "vite")
    print(f"UI: http://localhost:{args.vite_port}  (API on :{args.port}, HMR enabled)")


def cmd_stop(args):
    stopped = False
    for name in ("vite", "server"):
        pid = read_pid(name)
        if alive(pid):
            try:
                pgid = os.getpgid(pid)
                os.killpg(pgid, signal.SIGTERM)
                for _ in range(20):
                    if not alive(pid):
                        break
                    time.sleep(0.25)
                if alive(pid):
                    os.killpg(pgid, signal.SIGKILL)
                print(f"stopped {name} (pid {pid})")
                stopped = True
            except ProcessLookupError:
                pass
        pidfile(name).unlink(missing_ok=True)
    if not stopped:
        print("nothing was running")


def cmd_restart(args):
    cmd_stop(args)
    cmd_start(args)


def cmd_status(args):
    out = {}
    for name in ("server", "vite"):
        pid = read_pid(name)
        out[name] = {
            "pid": pid,
            "alive": alive(pid),
            "stale_pidfile": pid is not None and not alive(pid),
        }
    try:
        out["health"] = api("GET", "/api/health", timeout=3)
    except Exception as e:
        out["health"] = {"ok": False, "error": str(e)}
    out["urls"] = {
        "api": f"http://localhost:{DEFAULT_PORT}",
        "ui_dev": f"http://localhost:{DEFAULT_VITE_PORT}" if out["vite"]["alive"] else None,
    }
    print(json.dumps(out, indent=2))


def cmd_logs(args):
    tail_log(args.which, args.n)


def cmd_build(args):
    ensure_node()
    ensure_node_modules()
    result = subprocess.run(["npm", "run", "build"], cwd=WWW)
    if result.returncode != 0:
        die("vite build failed — see output above")


def _doc_file(name: str) -> Path:
    if "/" in name or name.startswith("."):
        die(f"invalid document name: {name!r}")
    return DATA / f"{name}.json"


def cmd_data(args):
    up = server_up()
    if args.action == "list":
        if up:
            print(json.dumps(api("GET", "/api/data")["data"], indent=2))
        else:
            docs = [p.stem for p in sorted(DATA.glob("*.json"))] if DATA.exists() else []
            print(json.dumps(docs, indent=2))
    elif args.action == "get":
        if up:
            try:
                print(json.dumps(api("GET", f"/api/data/{args.name}")["data"], indent=2))
            except urllib.error.HTTPError as e:
                die(f"GET /api/data/{args.name} → {e.code}: {e.read().decode(errors='replace')}")
        else:
            p = _doc_file(args.name)
            if not p.exists():
                die(f"no document {args.name!r} (server down, read from {DATA})")
            print(p.read_text())
    elif args.action == "set":
        if args.json is not None:
            raw = args.json
        elif args.file == "-":
            raw = sys.stdin.read()
        elif args.file:
            raw = Path(args.file).read_text()
        else:
            die("data set needs --json, --file, or --file - (stdin)")
        try:
            payload = json.loads(raw)
        except json.JSONDecodeError as e:
            die(f"not valid JSON: {e}")
        if up:
            api("PUT", f"/api/data/{args.name}", body=payload)
        else:
            p = _doc_file(args.name)
            DATA.mkdir(parents=True, exist_ok=True)
            tmp = p.with_suffix(".json.tmp")
            tmp.write_text(json.dumps(payload, indent=2))
            tmp.replace(p)
        print(f"saved {args.name}")
    elif args.action == "delete":
        if up:
            api("DELETE", f"/api/data/{args.name}")
        else:
            _doc_file(args.name).unlink(missing_ok=True)
        print(f"deleted {args.name}")


def cmd_call(args):
    body = None
    if args.json is not None:
        try:
            body = json.loads(args.json)
        except json.JSONDecodeError as e:
            die(f"--json is not valid JSON: {e}")
    try:
        print(json.dumps(api(args.method.upper(), args.path, body=body), indent=2))
    except urllib.error.HTTPError as e:
        die(f"{args.method.upper()} {args.path} → {e.code}: {e.read().decode(errors='replace')}")
    except urllib.error.URLError as e:
        die(f"server unreachable on :{DEFAULT_PORT} ({e.reason}) — `playpen start` first?")


def main():
    parser = argparse.ArgumentParser(prog="playpen", description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)

    for verb in ("start", "restart"):
        p = sub.add_parser(verb)
        p.add_argument("--build", action="store_true",
                       help="vite build, then serve dist/ from FastAPI alone")
        p.add_argument("--port", type=int, default=DEFAULT_PORT)
        p.add_argument("--vite-port", type=int, default=DEFAULT_VITE_PORT)
        p.set_defaults(func=cmd_start if verb == "start" else cmd_restart)

    sub.add_parser("stop").set_defaults(func=cmd_stop)
    sub.add_parser("status").set_defaults(func=cmd_status)
    sub.add_parser("build").set_defaults(func=cmd_build)

    p = sub.add_parser("logs")
    p.add_argument("which", nargs="?", choices=["server", "vite"], default="server")
    p.add_argument("-n", type=int, default=40)
    p.set_defaults(func=cmd_logs)

    p = sub.add_parser("data")
    dsub = p.add_subparsers(dest="action", required=True)
    dsub.add_parser("list")
    g = dsub.add_parser("get")
    g.add_argument("name")
    s = dsub.add_parser("set")
    s.add_argument("name")
    s.add_argument("--json")
    s.add_argument("--file")
    d = dsub.add_parser("delete")
    d.add_argument("name")
    p.set_defaults(func=cmd_data)

    p = sub.add_parser("call")
    p.add_argument("method")
    p.add_argument("path")
    p.add_argument("--json")
    p.set_defaults(func=cmd_call)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
