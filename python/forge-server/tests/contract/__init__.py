"""The Forge contract corpus, read in Python.

``contract/corpus.json`` is the contract as authored data; this package is what
a Python driver needs to run it — a typed reading of the file and a matcher for
the expectations it holds. ``crates/forge-contract`` is the same thing in Rust.

Nothing here knows about HTTP. The driver lives in ``tests/test_corpus.py``.
"""

from .corpus import (
    ABSENT,
    DEFAULT_FIXTURE,
    PYTHON_HTTP,
    Auth,
    AwaitEventStep,
    AwaitFrameStep,
    AwaitHeartbeatStep,
    Case,
    Connect,
    ConnectStep,
    Corpus,
    CorpusError,
    Expect,
    Fixture,
    FixtureEvents,
    FixtureUser,
    Kind,
    Request,
    RequestStep,
    SendStep,
    Step,
    users_env,
)
from .matcher import MatchError, Vars, interpolate, interpolate_value, match_value

__all__ = [
    "ABSENT",
    "DEFAULT_FIXTURE",
    "PYTHON_HTTP",
    "Auth",
    "AwaitEventStep",
    "AwaitFrameStep",
    "AwaitHeartbeatStep",
    "Case",
    "Connect",
    "ConnectStep",
    "Corpus",
    "CorpusError",
    "Expect",
    "Fixture",
    "FixtureEvents",
    "FixtureUser",
    "Kind",
    "MatchError",
    "Request",
    "RequestStep",
    "SendStep",
    "Step",
    "Vars",
    "interpolate",
    "interpolate_value",
    "match_value",
    "users_env",
]
