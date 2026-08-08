"""A typed reading of ``contract/corpus.json``, and the rules that keep it
honest.

Nothing here knows about HTTP. A driver supplies the transport: it builds the
fixture, turns a :class:`Request` into whatever its transport sends, and hands
the response back to the matcher.

Reading is strict. An unknown field is an error, because a corpus that is
half-read is worse than one that fails to load: the unread half looks like
coverage.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any

# corpus.py -> contract -> tests -> forge-server -> python -> the repo root
CORPUS_PATH = Path(__file__).resolve().parents[4] / "contract" / "corpus.json"

#: An expectation the case did not author. ``null`` is itself a matcher, so
#: absence needs a value of its own.
ABSENT = object()

#: Transport id of the Python HTTP driver.
PYTHON_HTTP = "python-http"


class CorpusError(Exception):
    """A corpus that cannot be read, or cannot be run honestly."""


class Kind(str, Enum):
    """What a case exercises, which decides the shape of its steps."""

    HTTP = "http"
    SSE = "sse"
    WS = "ws"


class Auth(str, Enum):
    """How a request carries its identity."""

    NONE = "none"
    BEARER = "bearer"
    QUERY = "query"


@dataclass(frozen=True)
class FixtureUser:
    name: str
    #: Plaintext. How a driver stores it is its own business.
    password: str
    roles: list[str]


@dataclass(frozen=True)
class FixtureAuth:
    #: Auth on. Off means every endpoint is open and the identity is anonymous.
    enabled: bool
    users: list[FixtureUser]


@dataclass(frozen=True)
class FixtureComponents:
    #: Written to ``manifest.json`` in the components directory.
    manifest: Any
    #: Written beside it: filename to content.
    files: dict[str, str]


@dataclass(frozen=True)
class FixtureFrontend:
    #: Written to the static frontend directory: filename to content.
    files: dict[str, str]


@dataclass(frozen=True)
class Fixture:
    """The server state the cases assume. See ``contract/README.md`` for the
    behaviour each named action must have."""

    app: str
    auth: FixtureAuth
    docstore: bool
    events: bool
    actions: list[str]
    components: FixtureComponents
    frontend: FixtureFrontend


@dataclass(frozen=True)
class Request:
    """One request, as it goes on the wire."""

    method: str
    #: A raw URI path, sent verbatim — already percent-encoded where it needs
    #: to be.
    path: str
    #: Query parameters. The driver encodes the values.
    query: dict[str, str]
    #: Extra headers, on top of whatever ``auth`` adds.
    headers: dict[str, str]
    auth: Auth
    #: A JSON request body, sent with a JSON content type. ``None`` = no body.
    body: Any


@dataclass(frozen=True)
class Connect:
    """A websocket handshake."""

    path: str
    query: dict[str, str]
    auth: Auth


@dataclass(frozen=True)
class Expect:
    """What must come back."""

    #: The HTTP status. A transport without status lines maps it through the
    #: contract's error kinds — see ``contract/README.md``.
    status: int
    #: Header name (lower-case) to a matcher over its value.
    headers: dict[str, Any]
    #: Matcher over the parsed JSON body, or :data:`ABSENT`.
    body: Any
    #: Matcher over the raw body for a response that is not JSON, or
    #: :data:`ABSENT`.
    text: Any


@dataclass(frozen=True)
class RequestStep:
    """Send one request and check the response."""

    request: Request
    expect: Expect | None


@dataclass(frozen=True)
class ConnectStep:
    """Open a websocket. An ``expect`` means the handshake must be refused."""

    connect: Connect
    expect: Expect | None


@dataclass(frozen=True)
class SendStep:
    """Send a JSON frame on the open socket."""

    frame: Any


@dataclass(frozen=True)
class AwaitFrameStep:
    """The next frame on the socket must match."""

    matcher: Any


@dataclass(frozen=True)
class AwaitEventStep:
    """The next event on the stream must have this topic and match this data."""

    topic: str
    data: Any


Step = RequestStep | ConnectStep | SendStep | AwaitFrameStep | AwaitEventStep


@dataclass(frozen=True)
class Case:
    """One contract case."""

    id: str
    title: str
    kind: Kind
    #: Why the case is written the way it is. Not asserted on.
    note: str | None
    #: Transports that must run this case.
    applies: list[str]
    #: Transports that cannot serve it, and what stops them.
    inapplicable: dict[str, str]
    steps: list[Step] = field(default_factory=list)

    def applies_to(self, transport: str) -> bool:
        """Whether this transport must run the case."""
        return transport in self.applies


@dataclass(frozen=True)
class Corpus:
    """One authored contract corpus."""

    #: Contract version the cases describe (``docs/api-contract.md``).
    contract_version: str
    #: Every transport a case must account for.
    transports: list[str]
    #: Substitution table for ``${name}`` in paths, bodies and expectations.
    vars: dict[str, str]
    #: The server every driver must build before running a case.
    fixture: Fixture
    #: The contract cases, in authored order.
    cases: list[Case]

    @classmethod
    def load(cls, path: Path = CORPUS_PATH) -> "Corpus":
        """Parse and validate the authored corpus."""
        try:
            raw = path.read_text()
        except OSError as e:
            raise CorpusError(f"cannot read {path}: {e}") from e
        return cls.parse(raw)

    @classmethod
    def parse(cls, text: str) -> "Corpus":
        """Parse and validate a corpus from JSON."""
        try:
            raw = json.loads(text)
        except json.JSONDecodeError as e:
            raise CorpusError(f"corpus is not readable: {e}") from e
        corpus = _corpus(raw)
        corpus.validate()
        return corpus

    def cases_for(self, transport: str) -> list[Case]:
        """Cases a transport must run, in authored order."""
        return [case for case in self.cases if case.applies_to(transport)]

    def validate(self) -> None:
        """Reject a corpus that cannot be run honestly.

        The rule that matters: every case accounts for every transport, so a
        coverage gap has to be written down rather than left out.
        """
        if not self.transports:
            raise CorpusError("corpus declares no transports")
        seen: set[str] = set()
        for case in self.cases:
            if case.id in seen:
                raise CorpusError(f"duplicate case id {case.id!r}")
            seen.add(case.id)
            self._validate_applicability(case)
            self._validate_steps(case)

    def _validate_applicability(self, case: Case) -> None:
        for transport in case.applies:
            if transport not in self.transports:
                raise CorpusError(
                    f"case {case.id!r} applies to unknown transport {transport!r}"
                )
        for transport, reason in case.inapplicable.items():
            if transport not in self.transports:
                raise CorpusError(
                    f"case {case.id!r} excuses unknown transport {transport!r}"
                )
            if not reason.strip():
                raise CorpusError(f"case {case.id!r} excuses {transport!r} with no reason")
            if case.applies_to(transport):
                raise CorpusError(
                    f"case {case.id!r} both applies to and excuses {transport!r}"
                )
        for transport in self.transports:
            if not case.applies_to(transport) and transport not in case.inapplicable:
                raise CorpusError(
                    f"case {case.id!r} says nothing about transport {transport!r} — "
                    "list it under `applies` or give a reason under `inapplicable`"
                )

    def _validate_steps(self, case: Case) -> None:
        if not case.steps:
            raise CorpusError(f"case {case.id!r} has no steps")
        first = case.steps[0]
        if case.kind is Kind.HTTP:
            for step in case.steps:
                if not isinstance(step, RequestStep):
                    raise CorpusError(
                        f"case {case.id!r} is kind `http`, so every step must be a request"
                    )
        elif case.kind is Kind.SSE:
            if not isinstance(first, RequestStep):
                raise CorpusError(
                    f"case {case.id!r} is kind `sse`, so its first step must be the "
                    "request that opens the stream"
                )
            # The stream's own response has no body to read — it is the stream.
            # A driver would drop a body expectation authored here without a
            # word, which is the silent gap this corpus exists to stop. Assert
            # on the events instead, with `await_event`.
            authored = first.expect is not None and (
                first.expect.body is not ABSENT or first.expect.text is not ABSENT
            )
            if authored:
                raise CorpusError(
                    f"case {case.id!r} expects a body from the request that opens the "
                    "stream; only its status and headers can be checked"
                )
            for step in case.steps:
                if isinstance(step, (ConnectStep, SendStep)):
                    raise CorpusError(
                        f"case {case.id!r} is kind `sse`; a stream cannot be connected "
                        "to or sent on"
                    )
        else:
            if not isinstance(first, ConnectStep):
                raise CorpusError(
                    f"case {case.id!r} is kind `ws`, so its first step must be a connect"
                )
            for step in case.steps[1:]:
                if isinstance(step, (ConnectStep, AwaitEventStep)):
                    raise CorpusError(
                        f"case {case.id!r} connects once, and awaits frames rather "
                        "than events"
                    )


# -- reading ---------------------------------------------------------------
#
# One helper per shape, so an unreadable corpus says which field it tripped on.


def _fields(raw: Any, where: str, known: set[str]) -> dict:
    if not isinstance(raw, dict):
        raise CorpusError(f"{where}: expected an object, got {type(raw).__name__}")
    unknown = sorted(set(raw) - known)
    if unknown:
        raise CorpusError(f"{where}: unknown field(s) {', '.join(repr(k) for k in unknown)}")
    return raw


def _required(raw: dict, key: str, where: str, kind: type | tuple[type, ...]) -> Any:
    if key not in raw:
        raise CorpusError(f"{where}: missing {key!r}")
    return _typed(raw[key], f"{where}.{key}", kind)


def _optional(raw: dict, key: str, where: str, kind: type | tuple[type, ...], default: Any) -> Any:
    if key not in raw:
        return default
    return _typed(raw[key], f"{where}.{key}", kind)


def _typed(value: Any, where: str, kind: type | tuple[type, ...]) -> Any:
    if kind is not Any and not isinstance(value, kind):
        names = kind if isinstance(kind, tuple) else (kind,)
        raise CorpusError(
            f"{where}: expected {' or '.join(k.__name__ for k in names)}, "
            f"got {type(value).__name__}"
        )
    return value


def _str_map(raw: Any, where: str) -> dict[str, str]:
    mapping = _typed(raw, where, dict)
    for key, value in mapping.items():
        _typed(value, f"{where}.{key}", str)
    return dict(mapping)


def _str_list(raw: Any, where: str) -> list[str]:
    items = _typed(raw, where, list)
    return [_typed(item, f"{where}[{i}]", str) for i, item in enumerate(items)]


def _enum(raw: Any, where: str, kind: type[Enum]) -> Any:
    value = _typed(raw, where, str)
    try:
        return kind(value)
    except ValueError:
        allowed = ", ".join(repr(member.value) for member in kind)
        raise CorpusError(f"{where}: {value!r} is not one of {allowed}") from None


def _corpus(raw: Any) -> Corpus:
    obj = _fields(raw, "corpus", {"contract_version", "transports", "vars", "fixture", "cases"})
    cases = _required(obj, "cases", "corpus", list)
    return Corpus(
        contract_version=_required(obj, "contract_version", "corpus", str),
        transports=_str_list(_required(obj, "transports", "corpus", list), "corpus.transports"),
        vars=_str_map(_required(obj, "vars", "corpus", dict), "corpus.vars"),
        fixture=_fixture(_required(obj, "fixture", "corpus", dict)),
        cases=[_case(item, i) for i, item in enumerate(cases)],
    )


def _fixture(raw: Any) -> Fixture:
    where = "fixture"
    obj = _fields(
        raw, where, {"app", "auth", "docstore", "events", "actions", "components", "frontend"}
    )
    components = _fields(
        _required(obj, "components", where, dict), f"{where}.components", {"manifest", "files"}
    )
    frontend = _fields(
        _required(obj, "frontend", where, dict), f"{where}.frontend", {"files"}
    )
    return Fixture(
        app=_required(obj, "app", where, str),
        auth=_fixture_auth(_required(obj, "auth", where, dict)),
        docstore=_required(obj, "docstore", where, bool),
        events=_required(obj, "events", where, bool),
        actions=_str_list(_required(obj, "actions", where, list), f"{where}.actions"),
        components=FixtureComponents(
            manifest=_required(components, "manifest", f"{where}.components", Any),
            files=_str_map(
                _required(components, "files", f"{where}.components", dict),
                f"{where}.components.files",
            ),
        ),
        frontend=FixtureFrontend(
            files=_str_map(
                _required(frontend, "files", f"{where}.frontend", dict),
                f"{where}.frontend.files",
            )
        ),
    )


def _fixture_auth(raw: Any) -> FixtureAuth:
    where = "fixture.auth"
    obj = _fields(raw, where, {"enabled", "users"})
    users = _required(obj, "users", where, list)
    return FixtureAuth(
        enabled=_required(obj, "enabled", where, bool),
        users=[_fixture_user(user, i) for i, user in enumerate(users)],
    )


def _fixture_user(raw: Any, index: int) -> FixtureUser:
    where = f"fixture.auth.users[{index}]"
    obj = _fields(raw, where, {"name", "password", "roles"})
    return FixtureUser(
        name=_required(obj, "name", where, str),
        password=_required(obj, "password", where, str),
        roles=_str_list(_optional(obj, "roles", where, list, []), f"{where}.roles"),
    )


def _case(raw: Any, index: int) -> Case:
    where = f"cases[{index}]"
    obj = _fields(
        raw, where, {"id", "title", "kind", "note", "applies", "inapplicable", "steps"}
    )
    case_id = _required(obj, "id", where, str)
    steps = _required(obj, "steps", where, list)
    return Case(
        id=case_id,
        title=_required(obj, "title", where, str),
        kind=_enum(_optional(obj, "kind", where, str, "http"), f"{where}.kind", Kind),
        note=_optional(obj, "note", where, str, None),
        applies=_str_list(_required(obj, "applies", where, list), f"{where}.applies"),
        inapplicable=_str_map(
            _optional(obj, "inapplicable", where, dict, {}), f"{where}.inapplicable"
        ),
        steps=[_step(step, f"case {case_id!r} step {i}") for i, step in enumerate(steps)],
    )


def _step(raw: Any, where: str) -> Step:
    if not isinstance(raw, dict):
        raise CorpusError(f"{where}: expected an object, got {type(raw).__name__}")
    if "request" in raw:
        obj = _fields(raw, where, {"request", "expect"})
        return RequestStep(
            request=_request(_required(obj, "request", where, dict), where),
            expect=_expect_or_none(obj, where),
        )
    if "connect" in raw:
        obj = _fields(raw, where, {"connect", "expect"})
        return ConnectStep(
            connect=_connect(_required(obj, "connect", where, dict), where),
            expect=_expect_or_none(obj, where),
        )
    if "send" in raw:
        return SendStep(frame=_fields(raw, where, {"send"})["send"])
    if "await_frame" in raw:
        return AwaitFrameStep(matcher=_fields(raw, where, {"await_frame"})["await_frame"])
    if "await_event" in raw:
        obj = _fields(raw, where, {"await_event"})
        event = _fields(
            _required(obj, "await_event", where, dict), f"{where}.await_event", {"topic", "data"}
        )
        return AwaitEventStep(
            topic=_required(event, "topic", f"{where}.await_event", str),
            data=_required(event, "data", f"{where}.await_event", Any),
        )
    raise CorpusError(
        f"{where}: not a step — expected one of `request`, `connect`, `send`, "
        "`await_frame`, `await_event`"
    )


def _request(raw: Any, where: str) -> Request:
    where = f"{where}.request"
    obj = _fields(raw, where, {"method", "path", "query", "headers", "auth", "body"})
    if "body" in obj and obj["body"] is None:
        raise CorpusError(f"{where}: a request with no body omits `body`")
    return Request(
        method=_required(obj, "method", where, str),
        path=_required(obj, "path", where, str),
        query=_str_map(_optional(obj, "query", where, dict, {}), f"{where}.query"),
        headers=_str_map(_optional(obj, "headers", where, dict, {}), f"{where}.headers"),
        auth=_enum(_optional(obj, "auth", where, str, "none"), f"{where}.auth", Auth),
        body=obj.get("body"),
    )


def _connect(raw: Any, where: str) -> Connect:
    where = f"{where}.connect"
    obj = _fields(raw, where, {"path", "query", "auth"})
    return Connect(
        path=_required(obj, "path", where, str),
        query=_str_map(_optional(obj, "query", where, dict, {}), f"{where}.query"),
        auth=_enum(_optional(obj, "auth", where, str, "none"), f"{where}.auth", Auth),
    )


def _expect_or_none(raw: dict, where: str) -> Expect | None:
    if "expect" not in raw:
        return None
    where = f"{where}.expect"
    obj = _fields(raw["expect"], where, {"status", "headers", "body", "text"})
    return Expect(
        status=_required(obj, "status", where, int),
        headers=_typed(_optional(obj, "headers", where, dict, {}), f"{where}.headers", dict),
        body=obj.get("body", ABSENT),
        text=obj.get("text", ABSENT),
    )
