"""The Python HTTP driver for the contract corpus (``contract/corpus.json``).

It builds the fixture the corpus describes, then runs every case that declares
``python-http`` under ``applies``. The case list lives in the corpus, not here —
this file only knows how to turn an authored request into an HTTP request and
hand the response back to the matcher.

The fixture runs on a real port. An in-process test client would be quicker,
but three of the things the corpus asserts — the status of a refused websocket
handshake, the framing of an event stream, and a path that reaches the server
percent-encoded — are properties of the wire, and a client that never writes
one cannot check them.

One case, one test: a divergence names itself.
"""

from __future__ import annotations

import contextlib
import json
import socket
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Iterator, Mapping
from urllib.parse import quote

import httpx
import pytest
import uvicorn
from websockets.exceptions import ConnectionClosed, InvalidStatus
from websockets.sync.client import connect as ws_connect

from contract import (
    ABSENT,
    PYTHON_HTTP,
    Auth,
    AwaitEventStep,
    AwaitFrameStep,
    AwaitHeartbeatStep,
    Case,
    ConnectStep,
    Corpus,
    Expect,
    Fixture,
    FixtureEvents,
    Kind,
    MatchError,
    Request,
    RequestStep,
    SendStep,
    Vars,
    interpolate,
    interpolate_value,
    match_value,
    users_env,
)
from forge_server import ForgeApp

#: How long any one wait may take: a handshake, a frame, an event, a response.
WAIT = 5.0

#: Driver-local: the corpus does not observe the signing secret.
SECRET = "0123456789abcdef0123456789abcdef"

CORPUS = Corpus.load()
CASES = CORPUS.cases_for(PYTHON_HTTP)


def test_the_corpus_reaches_this_transport():
    """A driver that runs nothing must not look like a driver that passes."""
    assert CASES, f"no corpus case applies to {PYTHON_HTTP}"


@pytest.mark.parametrize("case", CASES, ids=[case.id for case in CASES])
def test_case(harnesses, case: Case):
    harnesses(case.fixture).run(case)


class Failure(AssertionError):
    """A case this transport did not satisfy."""


@dataclass
class Response:
    status: int
    headers: Mapping[str, str]
    text: str

    def json(self) -> Any:
        try:
            return json.loads(self.text)
        except ValueError:
            return _NOT_JSON


_NOT_JSON = object()


@pytest.fixture(scope="session")
def harnesses(tmp_path_factory) -> Iterator[Callable[[str], "Harness"]]:
    """One server per fixture a case names, built the first time it is needed.

    A fixture whose cases are all inapplicable to this transport costs nothing.
    """
    built: dict[str, Harness] = {}

    def get(name: str) -> Harness:
        if name not in built:
            built[name] = Harness.build(
                CORPUS, name, tmp_path_factory.mktemp(f"corpus-{name}")
            )
        return built[name]

    try:
        yield get
    finally:
        for harness in built.values():
            harness.stop()


class Harness:
    """The fixture the corpus describes, plus the token every case borrows."""

    def __init__(self, server: uvicorn.Server, thread: threading.Thread, port: int, variables: Vars):
        self.server = server
        self.thread = thread
        self.base = f"http://127.0.0.1:{port}"
        self.ws_base = f"ws://127.0.0.1:{port}"
        self.vars = variables
        self.client = httpx.Client(timeout=WAIT)

    # -- building ----------------------------------------------------------

    @classmethod
    def build(cls, corpus: Corpus, name: str, root: Path) -> "Harness":
        fixture = corpus.fixtures[name]
        variables = dict(corpus.vars)
        data = root / "data"
        components = root / "components"
        frontend = root / "frontend"
        for path in (data, components, frontend):
            path.mkdir(parents=True, exist_ok=True)
        _write_files(frontend, fixture.frontend.files, variables)

        app = ForgeApp(interpolate(fixture.app, variables))
        if fixture.auth.enabled:
            _reject_roles(fixture)
            # Configured the way a deployment configures it: through the
            # FORGE_AUTH_USERS parser. Handing the store a name and a secret
            # directly would step over the parse, which is where an argon2
            # hash's commas land.
            app.auth(secret=SECRET, users=users_env(fixture.auth, variables))
        if fixture.docstore:
            app.with_docstore(data)
        if fixture.events is not None:
            app.with_events(**_events_kwargs(fixture.events))
        if fixture.components is not None:
            # An authored manifest is written; an absent one is the point of
            # the fixture that leaves it out.
            if fixture.components.manifest is not None:
                manifest = interpolate_value(fixture.components.manifest, variables)
                (components / "manifest.json").write_text(json.dumps(manifest, indent=2))
            _write_files(components, fixture.components.files, variables)
            app.with_components(components)
        _register_actions(app, fixture.actions)
        # Last, so the SPA catch-all sits behind every API route.
        app.serve_frontend(frontend, spa=True)

        server, thread, port = _serve(app)
        harness = cls(server, thread, port, variables)
        harness.vars["token"] = harness._login(fixture)
        return harness

    def stop(self) -> None:
        self.client.close()
        self.server.should_exit = True
        self.thread.join(timeout=WAIT)

    def _login(self, fixture: Fixture) -> str:
        """The one thing the driver does that the corpus does not describe: it
        needs a token before it can run a case that carries one."""
        if not fixture.auth.enabled:
            return ""
        user = fixture.auth.users[0]
        res = self.client.post(
            f"{self.base}/api/auth/login",
            json={
                "username": interpolate(user.name, self.vars),
                "password": interpolate(user.password, self.vars),
            },
        )
        assert res.status_code == 200, f"fixture login: {res.text}"
        token = res.json()["data"]["token"]
        assert isinstance(token, str) and token, "login returns a token"
        return token

    # -- running -----------------------------------------------------------

    def run(self, case: Case) -> None:
        if case.kind is Kind.HTTP:
            self._run_http(case)
        elif case.kind is Kind.SSE:
            self._run_sse(case)
        else:
            self._run_ws(case)

    def _run_http(self, case: Case) -> None:
        for index, step in enumerate(case.steps):
            if not isinstance(step, RequestStep):
                raise Failure(f"step {index}: kind `http` takes request steps only")
            self._request(step, index)

    def _run_sse(self, case: Case) -> None:
        """The first step opens the stream and its response is checked for
        status and headers; later request steps go out beside it."""
        with contextlib.ExitStack() as stack:
            stream: _SseStream | None = None
            for index, step in enumerate(case.steps):
                if isinstance(step, RequestStep) and stream is None:
                    method, url, sent = self._call(step.request)
                    response = stack.enter_context(self.client.stream(method, url, **sent))
                    head = Response(response.status_code, response.headers, "")
                    _check_status_and_headers(step.expect, head, index, self.vars)
                    stream = _SseStream(response)
                elif isinstance(step, RequestStep):
                    self._request(step, index)
                elif isinstance(step, AwaitEventStep):
                    assert stream is not None  # opened by the first step
                    self._expect_event(stream, step, index)
                elif isinstance(step, AwaitHeartbeatStep):
                    assert stream is not None  # opened by the first step
                    self._expect_heartbeat(stream, step, index)
                else:
                    raise Failure(f"step {index}: not a step a stream can take")

    def _run_ws(self, case: Case) -> None:
        socket_ = None
        try:
            for index, step in enumerate(case.steps):
                if isinstance(step, ConnectStep):
                    socket_ = self._connect(step, index)
                elif isinstance(step, SendStep):
                    frame = interpolate_value(step.frame, self.vars)
                    _open(socket_, index).send(json.dumps(frame))
                elif isinstance(step, AwaitFrameStep):
                    frame = _next_frame(_open(socket_, index), index)
                    _match(step.matcher, frame, self.vars, index)
                elif isinstance(step, RequestStep):
                    self._request(step, index)
                else:
                    raise Failure(f"step {index}: a socket awaits frames")
        finally:
            if socket_ is not None:
                socket_.close()

    def _connect(self, step: ConnectStep, index: int):
        url = self.ws_base + self._uri(step.connect.path, step.connect.query, step.connect.auth)
        try:
            socket_ = ws_connect(url, open_timeout=WAIT)
        except InvalidStatus as e:
            if step.expect is None:
                raise Failure(f"step {index}: handshake refused: {e}") from None
            refusal = Response(
                e.response.status_code,
                {name.lower(): value for name, value in e.response.headers.raw_items()},
                bytes(e.response.body or b"").decode("utf-8", "replace"),
            )
            self._check(step.expect, refusal, index)
            return None
        except OSError as e:
            raise Failure(f"step {index}: cannot open a socket: {e}") from None
        if step.expect is not None:
            socket_.close()
            raise Failure(f"step {index}: handshake succeeded, expected a refusal")
        return socket_

    def _request(self, step: RequestStep, index: int) -> None:
        method, url, sent = self._call(step.request)
        response = self.client.request(method, url, **sent)
        self._check(
            step.expect,
            Response(response.status_code, response.headers, response.text),
            index,
        )

    def _call(self, request: Request) -> tuple[str, str, dict[str, Any]]:
        """One authored request, as the method, URL and keywords httpx takes."""
        url = self.base + self._uri(request.path, request.query, request.auth)
        headers = {
            name: interpolate(value, self.vars) for name, value in request.headers.items()
        }
        if request.auth is Auth.BEARER:
            headers["authorization"] = f"Bearer {self._token()}"
        content = None
        if request.body is not None:
            headers.setdefault("content-type", "application/json")
            content = json.dumps(interpolate_value(request.body, self.vars)).encode()
        return request.method, url, {"headers": headers, "content": content}

    def _uri(self, path: str, query: Mapping[str, str], auth: Auth) -> str:
        """Path plus query. The path is authored as it goes on the wire, so it
        is used verbatim; query values are encoded here."""
        pairs = [
            f"{name}={quote(interpolate(value, self.vars), safe='')}"
            for name, value in query.items()
        ]
        if auth is Auth.QUERY:
            pairs.append(f"token={quote(self._token(), safe='')}")
        path = interpolate(path, self.vars)
        return f"{path}?{'&'.join(pairs)}" if pairs else path

    def _token(self) -> str:
        return self.vars.get("token", "")

    def _expect_event(self, stream: "_SseStream", step: AwaitEventStep, index: int) -> None:
        topic, data = stream.next_event(index)
        wanted = interpolate(step.topic, self.vars)
        if topic != wanted:
            raise Failure(f"step {index}: expected topic {wanted!r}, got {topic!r}")
        _match(step.data, data, self.vars, index)

    def _expect_heartbeat(
        self, stream: "_SseStream", step: AwaitHeartbeatStep, index: int
    ) -> None:
        """The *next* block, not the next heartbeat: a driver that read past an
        event to find one would pass while the stream carried something the
        contract does not allow."""
        block = stream.next_block(index)
        if isinstance(block, _Event):
            raise Failure(
                f"step {index}: expected a heartbeat, got the event {block.topic!r} "
                f"carrying {block.data!r}"
            )
        _match(step.matcher, block.text, self.vars, index)

    def _check(self, expect: Expect | None, response: Response, index: int) -> None:
        _check_status_and_headers(expect, response, index, self.vars)
        if expect is None:
            return
        if expect.body is not ABSENT:
            body = response.json()
            if body is _NOT_JSON:
                raise Failure(f"step {index}: body is not JSON: {response.text!r}")
            _match(expect.body, body, self.vars, index)
        if expect.text is not ABSENT:
            _match(expect.text, response.text, self.vars, index)


def _check_status_and_headers(
    expect: Expect | None, response: Response, index: int, variables: Vars
) -> None:
    if expect is None:
        return
    if response.status != expect.status:
        raise Failure(
            f"step {index}: expected status {expect.status}, got {response.status} "
            f"({response.text.strip()})"
        )
    for name, want in expect.headers.items():
        if name not in response.headers:
            raise Failure(f"step {index}: no {name} header")
        try:
            match_value(want, response.headers[name], variables)
        except MatchError as e:
            raise Failure(f"step {index}: header {name}: {e}") from None


def _match(expected: Any, actual: Any, variables: Vars, index: int) -> None:
    try:
        match_value(expected, actual, variables)
    except MatchError as e:
        raise Failure(f"step {index}: {e}") from None


def _open(socket_, index: int):
    if socket_ is None:
        raise Failure(f"step {index}: no open socket")
    return socket_


def _next_frame(socket_, index: int) -> Any:
    """The *next* frame, not the next matching one. A driver that searched
    forward would pass while the server sent frames the contract does not
    allow."""
    try:
        message = socket_.recv(timeout=WAIT)
    except TimeoutError:
        raise Failure(f"step {index}: timed out waiting for a frame") from None
    except ConnectionClosed:
        raise Failure(f"step {index}: the socket closed") from None
    if isinstance(message, bytes):
        message = message.decode("utf-8", "replace")
    try:
        return json.loads(message)
    except ValueError as e:
        raise Failure(f"step {index}: frame is not JSON: {message!r} ({e})") from None


@dataclass
class _Event:
    """An ``event:``/``data:`` pair off the stream."""

    topic: str
    data: Any


@dataclass
class _Heartbeat:
    """The comment that holds an idle stream open, without its colon."""

    text: str


class _SseStream:
    """Reads blocks off a live event stream."""

    def __init__(self, response: httpx.Response):
        self.chunks = response.iter_text()
        self.buffer = ""

    def next_event(self, index: int) -> tuple[str, Any]:
        """The next event. Comment heartbeats are not events, so they are
        stepped over — ``await_heartbeat`` is the step that sees one."""
        while True:
            block = self.next_block(index)
            if isinstance(block, _Event):
                return block.topic, block.data

    def next_block(self, index: int) -> "_Event | _Heartbeat":
        while True:
            block = self._take_block()
            if block is not None:
                parsed = _parse_sse_block(block, index)
                if parsed is not None:
                    return parsed
                continue
            try:
                self.buffer += next(self.chunks)
            except httpx.TimeoutException:
                raise Failure(f"step {index}: timed out waiting for the stream") from None
            except StopIteration:
                raise Failure(f"step {index}: the stream ended") from None

    def _take_block(self) -> str | None:
        # Frames end at a blank line; the line ending is the server's choice.
        buffer = self.buffer.replace("\r\n", "\n")
        end = buffer.find("\n\n")
        if end < 0:
            return None
        self.buffer = buffer[end + 2 :]
        return buffer[:end]


def _parse_sse_block(block: str, index: int) -> "_Event | _Heartbeat | None":
    """``None`` for a block that is neither an event nor a heartbeat."""
    topic = None
    data = None
    comment = None
    for line in block.splitlines():
        if line.startswith("event:"):
            topic = line[len("event:") :].strip()
        elif line.startswith("data:"):
            data = line[len("data:") :].strip()
        elif line.startswith(":") and comment is None:
            comment = line[1:].strip()
    if topic is None or data is None:
        return _Heartbeat(comment) if comment is not None else None
    try:
        return _Event(topic, json.loads(data))
    except ValueError as e:
        raise Failure(f"step {index}: event data is not JSON: {data!r} ({e})") from None


# -- the fixture -----------------------------------------------------------


def _serve(app: ForgeApp) -> tuple[uvicorn.Server, threading.Thread, int]:
    """Serve the fixture on a port the operating system chooses.

    The socket is bound here rather than by uvicorn, so the port cannot be
    taken between choosing it and listening on it.
    """
    listener = socket.socket()
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("127.0.0.1", 0))
    listener.listen()
    port = listener.getsockname()[1]

    server = uvicorn.Server(uvicorn.Config(app.fastapi, log_level="warning"))
    thread = threading.Thread(target=server.run, kwargs={"sockets": [listener]}, daemon=True)
    thread.start()

    deadline = time.monotonic() + WAIT
    while not server.started:
        if not thread.is_alive() or time.monotonic() > deadline:
            raise AssertionError("the corpus fixture did not come up")
        time.sleep(0.02)
    return server, thread, port


def _reject_roles(fixture: Fixture) -> None:
    """Loud rather than silent: this backend issues a token with no roles, and
    ``FORGE_AUTH_USERS`` carries none either, so a fixture that wants them is
    not the fixture served. The two Rust drivers put roles back on after the
    parse; this one cannot."""
    for user in fixture.auth.users:
        if user.roles:
            raise AssertionError(
                f"the corpus fixture gives {user.name!r} roles, which this backend "
                "cannot put in a token"
            )


def _events_kwargs(events: FixtureEvents) -> dict[str, Any]:
    """The event knobs the fixture tightens; the rest keep the backend's
    defaults, which are the contract's."""
    kwargs: dict[str, Any] = {}
    if events.buffer is not None:
        kwargs["buffer"] = events.buffer
    if events.heartbeat_s is not None:
        kwargs["heartbeat_s"] = events.heartbeat_s
    return kwargs


def _register_actions(app: ForgeApp, names: list[str]) -> None:
    """The behaviour each fixture action must have, per ``contract/README.md``.

    An action the corpus names and this driver has no behaviour for is a
    failure, not a silent gap.
    """
    for name in names:
        if name == "echo":
            app.action(name)(_echo)
        elif name == "publish":
            app.action(name)(_publish)
        elif name == "flood":
            app.action(name)(_flood)
        else:
            raise AssertionError(
                "the corpus fixture wants an action this driver has no behaviour "
                f"for: {name!r}"
            )


def _echo(payload: Any) -> Any:
    return payload


def _publish(payload: Any, ctx: Any) -> Any:
    topic = str(payload.get("topic", "misc"))
    ctx.events.publish(topic, payload.get("data"))
    return {"published": True, "topic": topic}


async def _flood(payload: Any, ctx: Any) -> Any:
    """Publish ``count`` events on ``topic`` in one go.

    Async, and awaiting nothing, so it runs to the end without giving a
    subscriber a turn: that is what makes overrunning a bounded buffer a
    certainty rather than a race.
    """
    topic = str(payload.get("topic", "misc"))
    count = int(payload.get("count", 0))
    for n in range(count):
        ctx.events.publish(topic, {"n": n})
    return {"published": count, "topic": topic}


def _write_files(directory: Path, files: Mapping[str, str], variables: Vars) -> None:
    for name, content in files.items():
        (directory / interpolate(name, variables)).write_text(
            interpolate(content, variables)
        )
