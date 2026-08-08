"""Serves contract-corpus fixtures for the TypeScript client driver.

The corpus states which servers a case runs against. This script builds them
the way the Python HTTP driver does — same ``Harness``, same parsers, same
port-zero binding — so the client is checked against a backend the corpus
already verifies. It prints one JSON line mapping each requested fixture name
to its port, then blocks until stdin closes, which is how the driver says it
is done.

Not named ``test_*``: pytest must not collect it.

Usage::

    uv run --project python/forge-server --extra dev \
        python python/forge-server/tests/corpus_fixture_server.py default auth-disabled
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

TESTS = Path(__file__).resolve().parent
sys.path.insert(0, str(TESTS))

from contract import Corpus  # noqa: E402
from test_corpus import Harness  # noqa: E402


def main(names: list[str]) -> None:
    if not names:
        raise SystemExit("usage: corpus_fixture_server.py <fixture> [<fixture> ...]")
    corpus = Corpus.load()
    unknown = [name for name in names if name not in corpus.fixtures]
    if unknown:
        raise SystemExit(
            f"the corpus has no fixture named {', '.join(repr(name) for name in unknown)}"
        )
    harnesses: list[Harness] = []
    with tempfile.TemporaryDirectory(prefix="corpus-ts-client-") as tmp:
        try:
            for name in names:
                root = Path(tmp) / name
                root.mkdir()
                harnesses.append(Harness.build(corpus, name, root))
            ports = {name: harness.port for name, harness in zip(names, harnesses)}
            print(json.dumps(ports), flush=True)
            sys.stdin.read()  # the driver closes stdin when it is done
        finally:
            for harness in harnesses:
                harness.stop()


if __name__ == "__main__":
    main(sys.argv[1:])
