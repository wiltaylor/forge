#!/usr/bin/env python3
"""Firing probes for wayfinder #75.

Each run is a fresh `claude` in a copy of one substrate. The skill lives in a
shadow CLAUDE_CONFIG_DIR alongside the 51 personal skills, so it is listed to the
model but invisible to `ls` in the working directory. Every path the agent can
see is free of the string "forge" for the plain-* substrates.

A run stops the moment the routing decision is made — a Skill call, or the first
Write/Edit, which means the agent chose to build without consulting anything.
"""
import json, os, pathlib, shutil, subprocess, sys, threading, queue

HERE = pathlib.Path(__file__).parent
WF = pathlib.Path("/tmp/claude-1000/wf")
CFG = WF / "cfg"
SUBSTRATES = WF / "substrates"
RUNS = WF / "runs"
OUT = HERE / "results"

MODEL = os.environ.get("PROBE_MODEL", "opus")
REPEATS = int(os.environ.get("PROBE_REPEATS", "3"))
TURNS = os.environ.get("PROBE_TURNS", "10")
BUDGET = os.environ.get("PROBE_BUDGET", "0.60")
CONCURRENCY = int(os.environ.get("PROBE_CONCURRENCY", "4"))

STOP_TOOLS = {"Write", "Edit", "NotebookEdit"}
# PROBE_FOLLOW=1 lets a run continue past the Skill call, to see what the agent
# does once SKILL.md is in front of it. Costs a full session per run.
FOLLOW = os.environ.get("PROBE_FOLLOW") == "1"


def one_run(probe, rep):
    rid = f"{probe['id']}--r{rep}"
    d = RUNS / rid
    if d.exists():
        shutil.rmtree(d)
    shutil.copytree(SUBSTRATES / probe["substrate"], d)

    cmd = [
        "claude", "-p", probe["ask"],
        "--model", MODEL,
        "--max-turns", TURNS,
        "--max-budget-usd", BUDGET,
        "--setting-sources", "user,project",
        "--strict-mcp-config",
        "--no-session-persistence",
        "--output-format", "stream-json", "--verbose",
    ]
    env = dict(os.environ, CLAUDE_CONFIG_DIR=str(CFG))

    rec = {
        "id": probe["id"], "rep": rep, "kind": probe["kind"],
        "substrate": probe["substrate"], "ask": probe["ask"],
        "expect": probe["expect"],
        "fired": False, "skill_invoked": None, "skill_args": None,
        "other_skills": [], "stopped_at": None,
        "listed": None, "text_before": [], "tools_before": [],
        "cost": None, "turns": None,
    }

    raw = []
    proc = subprocess.Popen(cmd, cwd=d, env=env,
                            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    try:
        for line in proc.stdout:
            raw.append(line)
            try:
                m = json.loads(line)
            except json.JSONDecodeError:
                continue
            t = m.get("type")
            if t == "system" and m.get("subtype") == "init":
                sc = m.get("slash_commands") or []
                rec["listed"] = "forge-design" in sc
            elif t == "assistant":
                for c in m["message"]["content"]:
                    if c["type"] == "text" and c["text"].strip():
                        rec["text_before"].append(c["text"])
                    elif c["type"] == "tool_use":
                        if c["name"] == "Skill":
                            s = c["input"].get("skill")
                            if s == "forge-design":
                                rec["fired"] = True
                                rec["skill_invoked"] = s
                                rec["skill_args"] = c["input"].get("args")
                                if not FOLLOW:
                                    rec["stopped_at"] = "skill"
                                    proc.terminate()
                                    raise StopIteration
                                continue
                            rec["other_skills"].append(s)
                        else:
                            rec["tools_before"].append(c["name"])
                            if c["name"] in STOP_TOOLS and not (FOLLOW and rec["fired"]):
                                rec["stopped_at"] = f"wrote-without-firing:{c['name']}"
                                proc.terminate()
                                raise StopIteration
            elif t == "result":
                rec["stopped_at"] = rec["stopped_at"] or m.get("subtype")
                rec["cost"] = m.get("total_cost_usd")
                rec["turns"] = m.get("num_turns")
    except StopIteration:
        pass
    finally:
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()

    (d / "_stream.jsonl").write_text("".join(raw))
    rec["run_dir"] = str(d)
    return rec


def main():
    probes = json.loads((HERE / "probes.json").read_text())["probes"]
    only = sys.argv[1:] or None
    if only:
        probes = [p for p in probes if p["id"] in only]

    jobs = [(p, r) for p in probes for r in range(1, REPEATS + 1)]
    q = queue.Queue()
    for j in jobs:
        q.put(j)
    results, lock = [], threading.Lock()

    def worker():
        while True:
            try:
                p, r = q.get_nowait()
            except queue.Empty:
                return
            try:
                rec = one_run(p, r)
            except Exception as e:  # a crashed run is data, not a stop
                rec = {"id": p["id"], "rep": r, "error": repr(e)}
            with lock:
                results.append(rec)
                mark = "FIRE" if rec.get("fired") else "----"
                print(f"[{len(results):>2}/{len(jobs)}] {mark} {p['id']} r{r} "
                      f"({rec.get('stopped_at')}, ${rec.get('cost')})", flush=True)

    threads = [threading.Thread(target=worker) for _ in range(CONCURRENCY)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()

    OUT.mkdir(exist_ok=True)
    results.sort(key=lambda r: (r["id"], r["rep"]))
    (OUT / "raw.json").write_text(json.dumps(results, indent=2))
    print(f"\nwrote {OUT / 'raw.json'}")


if __name__ == "__main__":
    main()
