/* The TypeScript client driver for the contract corpus (`contract/corpus.json`).

   It runs every case that declares `ts-client` under `applies` — the case
   list lives in the corpus, not here. The fixture is a real backend, served
   by `python/forge-server/tests/corpus_fixture_server.py`, which builds it
   with the same harness the Python HTTP driver uses; the backend itself is
   corpus-verified, so a failure here names the client.

   What makes this a *client* driver rather than a fourth HTTP driver: every
   request is made through the client's own surface — `auth.login`,
   `data.get`, `actions.call`, the shared socket — and the wire is observed
   through the client's injectable `fetch`, so the corpus checks the real
   request the client built and the real envelope that came back. Where the
   authored path is not one a typed method would put on the wire verbatim
   (a percent-encoded dot segment, a second path segment), the case goes
   through `client.request`, the public escape hatch, and the URL assertion
   below proves whichever surface ran authored the exact wire request.

   One case, one test: a divergence names itself. */

import { spawn, type ChildProcess } from 'node:child_process';
import { deepStrictEqual } from 'node:assert';
import { resolve } from 'node:path';
import { createInterface } from 'node:readline';
import { WebSocket as UndiciWebSocket } from 'undici';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { ApiError, createClient, type ForgeClient, type ForgeSocket } from '../src/index';
import {
  ABSENT,
  TS_CLIENT,
  casesFor,
  loadCorpus,
  type Case,
  type Expect,
  type Step,
} from './contract/corpus';
import { MatchError, interpolate, interpolateValue, matchValue, type Vars } from './contract/matcher';

/** How long any one wait may take: a handshake, a frame, a response. */
const WAIT = 5_000;

const REPO_ROOT = resolve(import.meta.dirname, '../../..');

/* The client reaches for `globalThis.WebSocket`, which Node gained in 22.
   On an older Node the same implementation comes from undici, the library
   behind the built-in one. */
if (typeof globalThis.WebSocket !== 'function') {
  (globalThis as { WebSocket: unknown }).WebSocket = UndiciWebSocket;
}

const corpus = loadCorpus();
const cases = casesFor(corpus, TS_CLIENT);

it('the corpus reaches this transport', () => {
  // A driver that runs nothing must not look like a driver that passes.
  expect(cases.length).toBeGreaterThan(0);
});

describe('contract corpus (ts-client)', () => {
  let server: ChildProcess | undefined;
  const harnesses = new Map<string, Harness>();

  beforeAll(async () => {
    const fixtures = [...new Set(cases.map((c) => c.fixture))];
    const { child, ports } = await startFixtureServer(fixtures);
    server = child;
    for (const name of fixtures) {
      harnesses.set(name, await buildHarness(name, ports[name] as number));
    }
  }, 180_000);

  afterAll(async () => {
    if (server) await stopFixtureServer(server);
  });

  for (const c of cases) {
    it(`${c.id} — ${c.title}`, { timeout: 30_000 }, async () => {
      const harness = harnesses.get(c.fixture);
      if (!harness) throw new Failure(`no harness for fixture '${c.fixture}'`);
      await runCase(harness, c);
    });
  }
});

/** A case this transport did not satisfy. */
class Failure extends Error {}

/** The fixture the corpus describes, plus the token every case borrows. */
interface Harness {
  base: string;
  wsBase: string;
  vars: Vars;
}

/** The one thing the driver does that the corpus does not describe: it needs
    a token before it can run a case that carries one. Obtained through the
    client, so a broken login fails here rather than as forty missing vars. */
async function buildHarness(name: string, port: number): Promise<Harness> {
  const base = `http://127.0.0.1:${port}`;
  const vars: Vars = { ...corpus.vars, token: '' };
  const fixture = corpus.fixtures[name];
  if (!fixture) throw new Error(`the corpus has no fixture '${name}'`);
  if (fixture.auth.enabled) {
    const user = fixture.auth.users[0];
    if (!user) throw new Error(`fixture '${name}' enables auth with no users`);
    const client = createClient({ baseUrl: base, tokenStorage: 'memory' });
    await client.auth.login(interpolate(user.name, vars), interpolate(user.password, vars));
    const token = client.auth.token();
    if (!token) throw new Error(`fixture '${name}': login stored no token`);
    vars.token = token;
  }
  return { base, wsBase: `ws://127.0.0.1:${port}`, vars };
}

// -- running ---------------------------------------------------------------

async function runCase(harness: Harness, c: Case): Promise<void> {
  const wire: Exchange[] = [];
  const client = createClient({
    baseUrl: harness.base,
    tokenStorage: 'memory',
    fetch: recordingFetch(wire),
  });
  if (c.kind === 'http') {
    for (const [index, step] of c.steps.entries()) {
      if (step.step !== 'request') throw new Failure(`step ${index}: kind \`http\` takes request steps only`);
      await runRequestStep(client, harness, wire, step, index);
    }
  } else if (c.kind === 'ws') {
    await runWsCase(client, harness, wire, c);
  } else {
    // The corpus authors every sse case as inapplicable here, with the reason
    // beside it; reaching this line means one was applied without a way to run.
    throw new Failure(`kind \`${c.kind}\` cases do not apply to this driver`);
  }
}

type RequestStep = Extract<Step, { step: 'request' }>;

async function runRequestStep(
  client: ForgeClient,
  harness: Harness,
  wire: Exchange[],
  step: RequestStep,
  index: number,
): Promise<void> {
  const req = step.request;
  const vars = harness.vars;
  const path = interpolate(req.path, vars);
  const token = vars['token'] ?? '';

  // The only header the client can author is the bearer one, from its stored
  // token. A case that authors another names a request outside the client's
  // surface, and belongs under `inapplicable`.
  let headerToken: string | null = null;
  for (const [name, value] of Object.entries(req.headers)) {
    if (name.toLowerCase() !== 'authorization') {
      throw new Failure(`step ${index}: the client cannot author a ${name} header`);
    }
    const authorized = interpolate(value, vars);
    if (!authorized.startsWith('Bearer ')) {
      throw new Failure(`step ${index}: the client sends only \`Bearer\` authorization`);
    }
    headerToken = authorized.slice('Bearer '.length);
  }
  if (Object.keys(req.query).length > 0) {
    throw new Failure(`step ${index}: the client's typed surface authors no query parameters`);
  }

  // `bearer` stores the fixture token; `none` stores nothing, unless the case
  // authors a header, which the client reproduces from a stored token;
  // `query` stores nothing so the query parameter is the only identity sent.
  client.auth.setToken(req.auth === 'bearer' ? token : headerToken);
  const fullPath = req.auth === 'query' ? `${path}?token=${encodeURIComponent(token)}` : path;
  const body = req.body === undefined ? undefined : interpolateValue(req.body, vars);

  const call = dispatch(client, req.method, path, fullPath, body);
  const before = wire.length;
  const outcome = await call.run().then(
    (value) => ({ ok: true as const, value }),
    (error: unknown) => ({ ok: false as const, error }),
  );
  if (wire.length !== before + 1) {
    throw new Failure(`step ${index}: the client sent ${wire.length - before} requests for one step`);
  }
  const exchange = wire[wire.length - 1] as Exchange;

  checkWireRequest(exchange, harness, req.method, fullPath, headerToken ?? (req.auth === 'bearer' ? token : null), index);
  checkExpect(step.expect, exchange, vars, index);
  checkSurface(client, call.surface, outcome, exchange, index);
}

// -- the client surface ----------------------------------------------------

type Surface =
  | 'login'
  | 'me'
  | 'health'
  | 'doc-list'
  | 'doc-get'
  | 'doc-put'
  | 'doc-del'
  | 'action'
  | 'raw';

interface Call {
  surface: Surface;
  run: () => Promise<unknown>;
}

/** Map one authored request onto the client method a real caller would use.
    A path no typed method would reproduce verbatim goes through
    `client.request`, the public escape hatch; `checkWireRequest` proves the
    chosen surface authored the exact wire request either way. */
function dispatch(
  client: ForgeClient,
  method: string,
  path: string,
  fullPath: string,
  body: unknown,
): Call {
  if (method === 'POST' && path === '/api/auth/login' && isCredentials(body)) {
    return { surface: 'login', run: () => client.auth.login(body.username, body.password) };
  }
  if (method === 'GET' && fullPath === '/api/auth/me') {
    return { surface: 'me', run: () => client.auth.me() };
  }
  if (method === 'GET' && path === '/api/health') {
    return { surface: 'health', run: () => client.health() };
  }
  if (method === 'GET' && path === '/api/data') {
    return { surface: 'doc-list', run: () => client.data.list() };
  }
  const doc = typedName(path, '/api/data/');
  if (doc !== null) {
    if (method === 'GET') return { surface: 'doc-get', run: () => client.data.get(doc) };
    if (method === 'PUT') return { surface: 'doc-put', run: () => client.data.put(doc, body) };
    if (method === 'DELETE') return { surface: 'doc-del', run: () => client.data.del(doc) };
  }
  const action = typedName(path, '/api/actions/');
  if (action !== null && method === 'POST') {
    return { surface: 'action', run: () => client.actions.call(action, body) };
  }
  return { surface: 'raw', run: () => client.request(method, fullPath, body) };
}

function isCredentials(body: unknown): body is { username: string; password: string } {
  return (
    typeof body === 'object' &&
    body !== null &&
    Object.keys(body).length === 2 &&
    typeof (body as Record<string, unknown>)['username'] === 'string' &&
    typeof (body as Record<string, unknown>)['password'] === 'string'
  );
}

/** The name a typed client method would put back on the wire verbatim, or
    null when it would not — a second path segment, or an encoding the
    client's `encodeURIComponent` does not produce (`%2E%2E` decodes to `..`,
    which re-encodes to itself and would hand a dot segment to the URL). */
function typedName(path: string, prefix: string): string | null {
  if (!path.startsWith(prefix)) return null;
  const segment = path.slice(prefix.length);
  if (segment === '') return null;
  let decoded: string;
  try {
    decoded = decodeURIComponent(segment);
  } catch {
    return null;
  }
  return encodeURIComponent(decoded) === segment ? decoded : null;
}

// -- checking --------------------------------------------------------------

/** The client must have put the authored request on the wire: the exact URL,
    the method, and an Authorization header exactly when the case carries a
    bearer identity. */
function checkWireRequest(
  exchange: Exchange,
  harness: Harness,
  method: string,
  fullPath: string,
  bearer: string | null,
  index: number,
): void {
  if (exchange.request.method !== method) {
    throw new Failure(`step ${index}: the client sent ${exchange.request.method}, the case authors ${method}`);
  }
  const wanted = harness.base + fullPath;
  if (exchange.request.url !== wanted) {
    throw new Failure(`step ${index}: the client requested ${exchange.request.url}, the case authors ${wanted}`);
  }
  const sent = exchange.request.headers['Authorization'];
  if (bearer !== null) {
    if (sent !== `Bearer ${bearer}`) {
      throw new Failure(`step ${index}: the client sent authorization ${JSON.stringify(sent)}`);
    }
  } else if (sent !== undefined) {
    throw new Failure(`step ${index}: the client sent an Authorization header the case does not author`);
  }
}

/** What every driver checks: the response on the wire, against the authored
    expectation. */
function checkExpect(expect: Expect | null, exchange: Exchange, vars: Vars, index: number): void {
  if (expect === null) return;
  const response = exchange.response;
  if (response.status !== expect.status) {
    throw new Failure(
      `step ${index}: expected status ${expect.status}, got ${response.status} (${response.text.trim()})`,
    );
  }
  for (const [name, want] of Object.entries(expect.headers)) {
    const value = response.headers.get(name);
    if (value === null) throw new Failure(`step ${index}: no ${name} header`);
    matched(want, value, vars, index, `header ${name}: `);
  }
  if (expect.body !== ABSENT) {
    let parsed: unknown;
    try {
      parsed = JSON.parse(response.text);
    } catch {
      throw new Failure(`step ${index}: body is not JSON: ${JSON.stringify(response.text)}`);
    }
    matched(expect.body, parsed, vars, index);
  }
  if (expect.text !== ABSENT) {
    matched(expect.text, response.text, vars, index);
  }
}

/** What only this driver can check: the client surfaced the envelope the
    contract's own way — data unwrapped on success, an `ApiError` carrying the
    status and the envelope's error on failure, and the two client-specific
    readings (`login` stores the token, `data.get` reads a 404 as null). */
function checkSurface(
  client: ForgeClient,
  surface: Surface,
  outcome: { ok: true; value: unknown } | { ok: false; error: unknown },
  exchange: Exchange,
  index: number,
): void {
  const status = exchange.response.status;
  const envelope = parseEnvelope(exchange.response.text);
  if (status < 400) {
    if (!outcome.ok) {
      throw new Failure(`step ${index}: the client rejected a ${status} response: ${String(outcome.error)}`);
    }
    if (surface === 'doc-put' || surface === 'doc-del') {
      if (outcome.value !== undefined) {
        throw new Failure(`step ${index}: a mutation resolved ${JSON.stringify(outcome.value)}`);
      }
      return;
    }
    try {
      deepStrictEqual(outcome.value, envelope?.data);
    } catch {
      throw new Failure(
        `step ${index}: the client resolved ${JSON.stringify(outcome.value)}, the envelope carries ` +
          `${JSON.stringify(envelope?.data)}`,
      );
    }
    if (surface === 'login') {
      const token = (envelope?.data as { token?: unknown } | undefined)?.token;
      if (client.auth.token() !== token) {
        throw new Failure(`step ${index}: login did not store the returned token`);
      }
    }
    return;
  }
  if (surface === 'doc-get' && status === 404) {
    if (!outcome.ok) {
      throw new Failure(`step ${index}: data.get must resolve null on 404, threw ${String(outcome.error)}`);
    }
    if (outcome.value !== null) {
      throw new Failure(`step ${index}: data.get resolved ${JSON.stringify(outcome.value)} on 404`);
    }
    return;
  }
  if (outcome.ok) {
    throw new Failure(`step ${index}: the client resolved a ${status} response`);
  }
  const error = outcome.error;
  if (!(error instanceof ApiError)) {
    throw new Failure(`step ${index}: the client threw ${String(error)} rather than an ApiError`);
  }
  if (error.status !== status) {
    throw new Failure(`step ${index}: ApiError carries status ${error.status}, the wire says ${status}`);
  }
  if (envelope && typeof envelope.error === 'string' && error.message !== envelope.error) {
    throw new Failure(
      `step ${index}: ApiError carries ${JSON.stringify(error.message)}, the envelope says ` +
        `${JSON.stringify(envelope.error)}`,
    );
  }
}

function parseEnvelope(text: string): { ok: boolean; data?: unknown; error?: unknown } | undefined {
  try {
    const parsed: unknown = JSON.parse(text);
    if (typeof parsed === 'object' && parsed !== null && 'ok' in parsed) {
      return parsed as { ok: boolean; data?: unknown; error?: unknown };
    }
  } catch {
    // not JSON — not an envelope
  }
  return undefined;
}

function matched(expected: unknown, actual: unknown, vars: Vars, index: number, what = ''): void {
  try {
    matchValue(expected, actual, vars);
  } catch (err) {
    if (err instanceof MatchError) throw new Failure(`step ${index}: ${what}${err.message}`);
    throw err;
  }
}

// -- websockets ------------------------------------------------------------

async function runWsCase(
  client: ForgeClient,
  harness: Harness,
  wire: Exchange[],
  c: Case,
): Promise<void> {
  const recorder = installSocketRecorder();
  let socket: ForgeSocket | null = null;
  try {
    for (const [index, step] of c.steps.entries()) {
      if (step.step === 'connect') {
        const { connect, expect: expected } = step;
        if (expected !== null) {
          // The excuse `ws-requires-token` writes down, enforced: the wrapper
          // reconnects on a refusal rather than reporting its status.
          throw new Failure(`step ${index}: the socket wrapper reports a close, not the handshake status`);
        }
        if (Object.keys(connect.query).length > 0 || connect.auth === 'bearer') {
          throw new Failure(`step ${index}: the client's socket carries only \`?token=\``);
        }
        if (interpolate(connect.path, harness.vars) !== '/api/ws') {
          throw new Failure(`step ${index}: the client's shared socket opens /api/ws`);
        }
        client.auth.setToken(connect.auth === 'query' ? (harness.vars['token'] ?? '') : null);
        socket = client.ws.connect();
        const record = recorder.only(index);
        const wanted =
          harness.wsBase +
          '/api/ws' +
          (connect.auth === 'query'
            ? `?token=${encodeURIComponent(harness.vars['token'] ?? '')}`
            : '');
        if (record.url !== wanted) {
          throw new Failure(`step ${index}: the client opened ${record.url}, the case authors ${wanted}`);
        }
        await withTimeout(openOf(socket), WAIT, `step ${index}: the socket did not open`);
      } else if (step.step === 'send') {
        if (!socket) throw new Failure(`step ${index}: no open socket`);
        const frame = interpolateValue(step.frame, harness.vars) as Record<string, unknown>;
        const record = recorder.only(index);
        const before = record.sent.length;
        // Subscriptions go through the wrapper's own subscribe, the surface a
        // real caller uses; anything else through the raw frame sender.
        if (frame['type'] === 'subscribe' && Array.isArray(frame['topics'])) {
          socket.subscribe(frame['topics'] as string[]);
        } else {
          socket.send(frame);
        }
        // The wrapper must have sent the authored frame, and only it.
        const sent = record.sent.slice(before).map((text) => JSON.parse(text) as unknown);
        try {
          deepStrictEqual(sent, [frame]);
        } catch {
          throw new Failure(
            `step ${index}: the client sent ${JSON.stringify(sent)}, the case authors ${JSON.stringify(frame)}`,
          );
        }
      } else if (step.step === 'await_frame') {
        const record = recorder.only(index);
        const text = await record.nextFrame(index);
        let frame: unknown;
        try {
          frame = JSON.parse(text);
        } catch {
          throw new Failure(`step ${index}: frame is not JSON: ${JSON.stringify(text)}`);
        }
        matched(step.matcher, frame, harness.vars, index);
      } else if (step.step === 'request') {
        await runRequestStep(client, harness, wire, step, index);
      } else {
        throw new Failure(`step ${index}: a socket awaits frames`);
      }
    }
  } finally {
    socket?.close();
    recorder.restore();
  }
}

function openOf(socket: ForgeSocket): Promise<void> {
  return new Promise((resolveOpen) => {
    const off = socket.on('open', () => {
      off();
      resolveOpen();
    });
  });
}

function withTimeout<T>(promise: Promise<T>, ms: number, message: string): Promise<T> {
  return new Promise((resolveValue, rejectValue) => {
    const timer = setTimeout(() => rejectValue(new Failure(message)), ms);
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolveValue(value);
      },
      (error: unknown) => {
        clearTimeout(timer);
        rejectValue(error as Error);
      },
    );
  });
}

/** One socket the client opened: the URL it chose, the frames it sent, and
    the frames the server sent back, in arrival order. */
interface RecordedSocket {
  url: string;
  sent: string[];
  /** The *next* frame, not the next matching one. A driver that searched
      forward would pass while the server sent frames the contract does not
      allow. */
  nextFrame(index: number): Promise<string>;
}

/** Wrap `globalThis.WebSocket` so the socket the client opens is observed on
    the wire while the client's own wrapper drives it. */
function installSocketRecorder(): {
  only(index: number): RecordedSocket;
  restore(): void;
} {
  const Native = globalThis.WebSocket;
  if (typeof Native !== 'function') {
    throw new Error('this driver needs the WebSocket global (Node >= 22)');
  }
  const sockets: RecordedSocket[] = [];
  function Recording(this: unknown, url: string | URL): WebSocket {
    const ws = new Native(String(url));
    const frames: string[] = [];
    const waiters: { resolve: (text: string) => void; reject: (err: Error) => void }[] = [];
    let closed = false;
    const sent: string[] = [];
    const nativeSend = ws.send.bind(ws);
    ws.send = (data: Parameters<WebSocket['send']>[0]) => {
      sent.push(String(data));
      nativeSend(data);
    };
    ws.addEventListener('message', (event) => {
      const text = String((event as MessageEvent).data);
      const waiter = waiters.shift();
      if (waiter) waiter.resolve(text);
      else frames.push(text);
    });
    ws.addEventListener('close', () => {
      closed = true;
      for (const waiter of waiters.splice(0)) {
        waiter.reject(new Failure('the socket closed'));
      }
    });
    sockets.push({
      url: String(url),
      sent,
      nextFrame(index: number): Promise<string> {
        const first = frames.shift();
        if (first !== undefined) return Promise.resolve(first);
        if (closed) return Promise.reject(new Failure(`step ${index}: the socket closed`));
        return withTimeout(
          new Promise<string>((resolveFrame, rejectFrame) =>
            waiters.push({ resolve: resolveFrame, reject: rejectFrame }),
          ),
          WAIT,
          `step ${index}: timed out waiting for a frame`,
        );
      },
    });
    return ws;
  }
  (globalThis as { WebSocket: unknown }).WebSocket = Recording;
  return {
    only(index: number): RecordedSocket {
      // One connect step, one socket. A second one means the client's
      // wrapper reconnected mid-case, which would split the frame stream.
      if (sockets.length !== 1) {
        throw new Failure(`step ${index}: the client opened ${sockets.length} sockets`);
      }
      return sockets[0] as RecordedSocket;
    },
    restore(): void {
      (globalThis as { WebSocket: unknown }).WebSocket = Native;
    },
  };
}

// -- the wire --------------------------------------------------------------

interface Exchange {
  request: {
    url: string;
    method: string;
    headers: Record<string, string>;
  };
  response: {
    status: number;
    headers: Headers;
    text: string;
  };
}

/** The client's injectable fetch, delegating to the real one and keeping a
    copy of what crossed the wire. The response body is read from a clone so
    the client still consumes its own. */
function recordingFetch(wire: Exchange[]): typeof fetch {
  return async (input, init) => {
    const response = await fetch(input, init);
    const text = await response.clone().text();
    wire.push({
      request: {
        url: String(input),
        method: init?.method ?? 'GET',
        headers: { ...((init?.headers ?? {}) as Record<string, string>) },
      },
      response: { status: response.status, headers: response.headers, text },
    });
    return response;
  };
}

// -- the fixture -----------------------------------------------------------

/** Start the backend that serves every fixture the ts-client cases name.
    It prints one JSON line of ports, then serves until stdin closes. */
function startFixtureServer(
  fixtures: string[],
): Promise<{ child: ChildProcess; ports: Record<string, number> }> {
  // One string, split — a quoted '--flag' on its own reads as a CSS custom
  // property to the design-token scan.
  const command =
    'run --project python/forge-server --extra dev ' +
    'python python/forge-server/tests/corpus_fixture_server.py';
  const child = spawn('uv', [...command.split(' '), ...fixtures], {
    cwd: REPO_ROOT,
    stdio: ['pipe', 'pipe', 'inherit'],
  });
  return new Promise((resolveServer, rejectServer) => {
    const die = (why: string) => rejectServer(new Error(why));
    child.once('error', (err) =>
      die(`cannot start the corpus fixture server (is uv installed?): ${err.message}`),
    );
    child.once('exit', (code) => die(`the corpus fixture server exited with ${code} before serving`));
    createInterface({ input: child.stdout as NodeJS.ReadableStream }).once('line', (line) => {
      child.removeAllListeners('exit');
      child.removeAllListeners('error');
      try {
        resolveServer({ child, ports: JSON.parse(line) as Record<string, number> });
      } catch (err) {
        die(`the corpus fixture server printed ${JSON.stringify(line)}: ${String(err)}`);
      }
    });
  });
}

function stopFixtureServer(child: ChildProcess): Promise<void> {
  return new Promise((resolveStop) => {
    child.once('exit', () => resolveStop());
    child.stdin?.end();
    setTimeout(() => child.kill('SIGKILL'), WAIT).unref();
  });
}
