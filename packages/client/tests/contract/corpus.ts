/* A typed reading of `contract/corpus.json`, and the rules that keep it
   honest.

   Nothing here knows about HTTP or about the client. A driver supplies the
   transport: it builds the fixture, turns a `CorpusRequest` into whatever its
   transport sends, and hands the response back to the matcher.

   Reading is strict on names: an unknown field anywhere is an error, because
   a corpus that is half-read is worse than one that fails to load — the
   unread half looks like coverage. This mirrors
   `python/forge-server/tests/contract/corpus.py` and
   `crates/forge-contract`, with one written-down narrowing: fixture *leaf
   values* this driver never consumes (buffer depths, file contents) are left
   to the backend that builds the fixture, which re-reads them strictly. */

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { isObject } from './matcher';

export const CORPUS_PATH = resolve(import.meta.dirname, '../../../../contract/corpus.json');

/** An expectation the case did not author. `null` is itself a matcher, so
    absence needs a value of its own. */
export const ABSENT: unique symbol = Symbol('absent');

/** Transport id of the TypeScript client driver. */
export const TS_CLIENT = 'ts-client';

/** Name of the fixture a case runs against unless it names another. */
export const DEFAULT_FIXTURE = 'default';

/** A corpus that cannot be read, or cannot be run honestly. */
export class CorpusError extends Error {}

/** What a case exercises, which decides the shape of its steps. */
export type Kind = 'http' | 'sse' | 'ws';

/** How a request carries its identity. */
export type AuthMode = 'none' | 'bearer' | 'query';

export interface FixtureUser {
  name: string;
  /** The plaintext a login sends. */
  password: string;
  /** How the backend stores the credential, in the `FORGE_AUTH_USERS` secret
      syntax. `null` means the password is stored as it stands. */
  secret: string | null;
  roles: string[];
}

export interface FixtureAuth {
  /** Auth on. Off means every endpoint is open and the identity is anonymous. */
  enabled: boolean;
  users: FixtureUser[];
}

/** The server state a case assumes. This driver hands the whole fixture to a
    backend that builds it, so only the parts the driver itself reads — the
    auth block it logs in with, and whether the event endpoints are mounted —
    are typed beyond their field names. */
export interface Fixture {
  auth: FixtureAuth;
  /** Whether the fixture mounts `/api/events` and `/api/ws`. */
  events: boolean;
}

/** One request, as it goes on the wire. */
export interface CorpusRequest {
  method: string;
  /** A raw URI path, sent verbatim — already percent-encoded where it needs
      to be. */
  path: string;
  /** Query parameters. The driver encodes the values. */
  query: Record<string, string>;
  /** Extra headers, on top of whatever `auth` adds. */
  headers: Record<string, string>;
  auth: AuthMode;
  /** A JSON request body, sent with a JSON content type. `undefined` = none;
      the corpus rejects an authored `null` body. */
  body?: unknown;
}

/** A websocket handshake. */
export interface Connect {
  path: string;
  query: Record<string, string>;
  auth: AuthMode;
}

/** What must come back. */
export interface Expect {
  status: number;
  /** Header name (lower-case) to a matcher over its value. */
  headers: Record<string, unknown>;
  /** Matcher over the parsed JSON body, or `ABSENT`. */
  body: unknown;
  /** Matcher over the raw body for a response that is not JSON, or `ABSENT`. */
  text: unknown;
}

export type Step =
  | { step: 'request'; request: CorpusRequest; expect: Expect | null }
  | { step: 'connect'; connect: Connect; expect: Expect | null }
  | { step: 'send'; frame: unknown }
  | { step: 'await_frame'; matcher: unknown }
  | { step: 'await_event'; topic: string; data: unknown }
  | { step: 'await_heartbeat'; matcher: unknown };

/** One contract case. */
export interface Case {
  id: string;
  title: string;
  kind: Kind;
  fixture: string;
  note: string | null;
  /** Transports that must run this case. */
  applies: string[];
  /** Transports that cannot serve it, and what stops them. */
  inapplicable: Record<string, string>;
  steps: Step[];
}

/** One authored contract corpus. */
export interface Corpus {
  contractVersion: string;
  transports: string[];
  vars: Record<string, string>;
  fixtures: Record<string, Fixture>;
  cases: Case[];
}

/** Parse and validate the authored corpus. */
export function loadCorpus(path: string = CORPUS_PATH): Corpus {
  return parseCorpus(readFileSync(path, 'utf8'));
}

/** Parse and validate a corpus from JSON. */
export function parseCorpus(text: string): Corpus {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (err) {
    throw new CorpusError(`corpus is not readable: ${String(err)}`);
  }
  const corpus = readCorpus(raw);
  validate(corpus);
  return corpus;
}

/** Cases a transport must run, in authored order. */
export function casesFor(corpus: Corpus, transport: string): Case[] {
  return corpus.cases.filter((c) => c.applies.includes(transport));
}

/* -- validation ----------------------------------------------------------
   Reject a corpus that cannot be run honestly. The rule that matters: every
   case accounts for every transport, so a coverage gap has to be written down
   rather than left out. */

function validate(corpus: Corpus): void {
  if (corpus.transports.length === 0) throw new CorpusError('corpus declares no transports');
  if (!(DEFAULT_FIXTURE in corpus.fixtures)) {
    throw new CorpusError(
      `corpus declares no '${DEFAULT_FIXTURE}' fixture — it is the one a case runs against ` +
        'unless it names another',
    );
  }
  const seen = new Set<string>();
  const used = new Set<string>();
  for (const c of corpus.cases) {
    if (seen.has(c.id)) throw new CorpusError(`duplicate case id '${c.id}'`);
    seen.add(c.id);
    if (!(c.fixture in corpus.fixtures)) {
      throw new CorpusError(`case '${c.id}' runs against unknown fixture '${c.fixture}'`);
    }
    used.add(c.fixture);
    validateApplicability(corpus, c);
    validateSteps(corpus, c);
  }
  // A fixture no case uses is a server every driver would build for nothing,
  // and reads as coverage that is not there.
  for (const name of Object.keys(corpus.fixtures)) {
    if (!used.has(name)) throw new CorpusError(`fixture '${name}' has no case`);
  }
}

function validateApplicability(corpus: Corpus, c: Case): void {
  for (const transport of c.applies) {
    if (!corpus.transports.includes(transport)) {
      throw new CorpusError(`case '${c.id}' applies to unknown transport '${transport}'`);
    }
  }
  for (const [transport, reason] of Object.entries(c.inapplicable)) {
    if (!corpus.transports.includes(transport)) {
      throw new CorpusError(`case '${c.id}' excuses unknown transport '${transport}'`);
    }
    if (reason.trim() === '') {
      throw new CorpusError(`case '${c.id}' excuses '${transport}' with no reason`);
    }
    if (c.applies.includes(transport)) {
      throw new CorpusError(`case '${c.id}' both applies to and excuses '${transport}'`);
    }
  }
  for (const transport of corpus.transports) {
    if (!c.applies.includes(transport) && !(transport in c.inapplicable)) {
      throw new CorpusError(
        `case '${c.id}' says nothing about transport '${transport}' — list it under ` +
          '`applies` or give a reason under `inapplicable`',
      );
    }
  }
}

function validateSteps(corpus: Corpus, c: Case): void {
  if (c.steps.length === 0) throw new CorpusError(`case '${c.id}' has no steps`);
  const first = c.steps[0] as Step;
  if (c.kind !== 'http' && !(corpus.fixtures[c.fixture] as Fixture).events) {
    throw new CorpusError(
      `case '${c.id}' is kind \`${c.kind}\`, but its fixture '${c.fixture}' mounts no event bus`,
    );
  }
  if (c.kind === 'http') {
    for (const step of c.steps) {
      if (step.step !== 'request') {
        throw new CorpusError(`case '${c.id}' is kind \`http\`, so every step must be a request`);
      }
    }
  } else if (c.kind === 'sse') {
    if (first.step !== 'request') {
      throw new CorpusError(
        `case '${c.id}' is kind \`sse\`, so its first step must be the request that opens ` +
          'the stream',
      );
    }
    // The stream's own response has no body to read — it is the stream. A
    // driver would drop a body expectation authored here without a word,
    // which is the silent gap this corpus exists to stop.
    const authored =
      first.expect !== null && (first.expect.body !== ABSENT || first.expect.text !== ABSENT);
    if (authored) {
      throw new CorpusError(
        `case '${c.id}' expects a body from the request that opens the stream; only its ` +
          'status and headers can be checked',
      );
    }
    for (const step of c.steps) {
      if (step.step !== 'request' && step.step !== 'await_event' && step.step !== 'await_heartbeat') {
        throw new CorpusError(
          `case '${c.id}' is kind \`sse\`; a stream cannot be connected to, sent on, or ` +
            'read a frame from',
        );
      }
    }
  } else {
    if (first.step !== 'connect') {
      throw new CorpusError(`case '${c.id}' is kind \`ws\`, so its first step must be a connect`);
    }
    for (const step of c.steps.slice(1)) {
      if (step.step === 'connect' || step.step === 'await_event' || step.step === 'await_heartbeat') {
        throw new CorpusError(
          `case '${c.id}' connects once, and awaits frames rather than events or heartbeats`,
        );
      }
    }
  }
}

/* -- reading -------------------------------------------------------------
   One helper per shape, so an unreadable corpus says which field it tripped
   on. */

function fields(raw: unknown, where: string, known: string[]): Record<string, unknown> {
  if (!isObject(raw)) {
    throw new CorpusError(`${where}: expected an object, got ${typeOf(raw)}`);
  }
  const unknown = Object.keys(raw)
    .filter((key) => !known.includes(key))
    .sort();
  if (unknown.length > 0) {
    throw new CorpusError(
      `${where}: unknown field(s) ${unknown.map((key) => `'${key}'`).join(', ')}`,
    );
  }
  return raw;
}

function required<T>(
  raw: Record<string, unknown>,
  key: string,
  where: string,
  kind: (value: unknown) => value is T,
  name: string,
): T {
  if (!(key in raw)) throw new CorpusError(`${where}: missing '${key}'`);
  return typed(raw[key], `${where}.${key}`, kind, name);
}

function optional<T>(
  raw: Record<string, unknown>,
  key: string,
  where: string,
  kind: (value: unknown) => value is T,
  name: string,
  fallback: T,
): T {
  if (!(key in raw)) return fallback;
  return typed(raw[key], `${where}.${key}`, kind, name);
}

function typed<T>(
  value: unknown,
  where: string,
  kind: (value: unknown) => value is T,
  name: string,
): T {
  if (!kind(value)) throw new CorpusError(`${where}: expected ${name}, got ${typeOf(value)}`);
  return value;
}

const isString = (value: unknown): value is string => typeof value === 'string';
const isBoolean = (value: unknown): value is boolean => typeof value === 'boolean';
const isInt = (value: unknown): value is number =>
  typeof value === 'number' && Number.isInteger(value);
const isList = (value: unknown): value is unknown[] => Array.isArray(value);

function typeOf(value: unknown): string {
  if (value === null) return 'null';
  if (Array.isArray(value)) return 'array';
  return typeof value;
}

function strMap(raw: unknown, where: string): Record<string, string> {
  const map = typed(raw, where, isObject, 'an object');
  for (const [key, value] of Object.entries(map)) {
    typed(value, `${where}.${key}`, isString, 'a string');
  }
  return map as Record<string, string>;
}

function strList(raw: unknown, where: string): string[] {
  const items = typed(raw, where, isList, 'an array');
  return items.map((item, i) => typed(item, `${where}[${i}]`, isString, 'a string'));
}

function oneOf<T extends string>(raw: unknown, where: string, allowed: readonly T[]): T {
  const value = typed(raw, where, isString, 'a string');
  if (!(allowed as readonly string[]).includes(value)) {
    throw new CorpusError(
      `${where}: '${value}' is not one of ${allowed.map((k) => `'${k}'`).join(', ')}`,
    );
  }
  return value as T;
}

function readCorpus(raw: unknown): Corpus {
  const obj = fields(raw, 'corpus', ['contract_version', 'transports', 'vars', 'fixtures', 'cases']);
  const fixturesRaw = required(obj, 'fixtures', 'corpus', isObject, 'an object');
  const casesRaw = required(obj, 'cases', 'corpus', isList, 'an array');
  const fixtures: Record<string, Fixture> = {};
  for (const [name, item] of Object.entries(fixturesRaw)) {
    fixtures[name] = readFixture(item, name);
  }
  return {
    contractVersion: required(obj, 'contract_version', 'corpus', isString, 'a string'),
    transports: strList(required(obj, 'transports', 'corpus', isList, 'an array'), 'corpus.transports'),
    vars: strMap(required(obj, 'vars', 'corpus', isObject, 'an object'), 'corpus.vars'),
    fixtures,
    cases: casesRaw.map((item, i) => readCase(item, i)),
  };
}

function readFixture(raw: unknown, name: string): Fixture {
  const where = `fixtures.${name}`;
  const obj = fields(raw, where, [
    'app',
    'auth',
    'docstore',
    'events',
    'actions',
    'components',
    'frontend',
  ]);
  // Unread parts still have their field names checked, so a typo cannot hide
  // in a corner this driver does not consume.
  if ('events' in obj) {
    fields(obj.events, `${where}.events`, ['buffer', 'heartbeat_s']);
  }
  if ('components' in obj) {
    fields(obj.components, `${where}.components`, ['manifest', 'files']);
  }
  if ('frontend' in obj) {
    fields(obj.frontend, `${where}.frontend`, ['files']);
  }
  return {
    auth: readAuth(required(obj, 'auth', where, isObject, 'an object'), where),
    events: 'events' in obj,
  };
}

function readAuth(raw: Record<string, unknown>, where: string): FixtureAuth {
  const at = `${where}.auth`;
  const obj = fields(raw, at, ['enabled', 'users']);
  const users = optional(obj, 'users', at, isList, 'an array', [] as unknown[]);
  return {
    enabled: required(obj, 'enabled', at, isBoolean, 'a boolean'),
    users: users.map((user, i) => readUser(user, i, at)),
  };
}

function readUser(raw: unknown, index: number, where: string): FixtureUser {
  const at = `${where}.users[${index}]`;
  const obj = fields(raw, at, ['name', 'password', 'secret', 'roles']);
  return {
    name: required(obj, 'name', at, isString, 'a string'),
    password: required(obj, 'password', at, isString, 'a string'),
    secret: optional<string | null>(obj, 'secret', at, isString, 'a string', null),
    roles: strList(optional(obj, 'roles', at, isList, 'an array', [] as unknown[]), `${at}.roles`),
  };
}

function readCase(raw: unknown, index: number): Case {
  const where = `cases[${index}]`;
  const obj = fields(raw, where, [
    'id',
    'title',
    'kind',
    'fixture',
    'note',
    'applies',
    'inapplicable',
    'steps',
  ]);
  const id = required(obj, 'id', where, isString, 'a string');
  const steps = required(obj, 'steps', where, isList, 'an array');
  return {
    id,
    title: required(obj, 'title', where, isString, 'a string'),
    kind: 'kind' in obj ? oneOf(obj.kind, `${where}.kind`, ['http', 'sse', 'ws']) : 'http',
    fixture: optional(obj, 'fixture', where, isString, 'a string', DEFAULT_FIXTURE),
    note: optional<string | null>(obj, 'note', where, isString, 'a string', null),
    applies: strList(required(obj, 'applies', where, isList, 'an array'), `${where}.applies`),
    inapplicable: strMap(
      optional(obj, 'inapplicable', where, isObject, 'an object', {}),
      `${where}.inapplicable`,
    ),
    steps: steps.map((step, i) => readStep(step, `case '${id}' step ${i}`)),
  };
}

function readStep(raw: unknown, where: string): Step {
  if (!isObject(raw)) throw new CorpusError(`${where}: expected an object, got ${typeOf(raw)}`);
  if ('request' in raw) {
    const obj = fields(raw, where, ['request', 'expect']);
    return {
      step: 'request',
      request: readRequest(required(obj, 'request', where, isObject, 'an object'), where),
      expect: readExpect(obj, where),
    };
  }
  if ('connect' in raw) {
    const obj = fields(raw, where, ['connect', 'expect']);
    return {
      step: 'connect',
      connect: readConnect(required(obj, 'connect', where, isObject, 'an object'), where),
      expect: readExpect(obj, where),
    };
  }
  if ('send' in raw) {
    return { step: 'send', frame: fields(raw, where, ['send']).send };
  }
  if ('await_frame' in raw) {
    return { step: 'await_frame', matcher: fields(raw, where, ['await_frame']).await_frame };
  }
  if ('await_event' in raw) {
    const obj = fields(raw, where, ['await_event']);
    const at = `${where}.await_event`;
    const event = fields(required(obj, 'await_event', where, isObject, 'an object'), at, [
      'topic',
      'data',
    ]);
    if (!('data' in event)) throw new CorpusError(`${at}: missing 'data'`);
    return {
      step: 'await_event',
      topic: required(event, 'topic', at, isString, 'a string'),
      data: event.data,
    };
  }
  if ('await_heartbeat' in raw) {
    return {
      step: 'await_heartbeat',
      matcher: fields(raw, where, ['await_heartbeat']).await_heartbeat,
    };
  }
  throw new CorpusError(
    `${where}: not a step — expected one of \`request\`, \`connect\`, \`send\`, ` +
      '`await_frame`, `await_event`, `await_heartbeat`',
  );
}

function readRequest(raw: Record<string, unknown>, where: string): CorpusRequest {
  const at = `${where}.request`;
  const obj = fields(raw, at, ['method', 'path', 'query', 'headers', 'auth', 'body']);
  if ('body' in obj && obj.body === null) {
    throw new CorpusError(`${at}: a request with no body omits \`body\``);
  }
  const request: CorpusRequest = {
    method: required(obj, 'method', at, isString, 'a string'),
    path: required(obj, 'path', at, isString, 'a string'),
    query: strMap(optional(obj, 'query', at, isObject, 'an object', {}), `${at}.query`),
    headers: strMap(optional(obj, 'headers', at, isObject, 'an object', {}), `${at}.headers`),
    auth: 'auth' in obj ? oneOf(obj.auth, `${at}.auth`, ['none', 'bearer', 'query']) : 'none',
  };
  if ('body' in obj) request.body = obj.body;
  return request;
}

function readConnect(raw: Record<string, unknown>, where: string): Connect {
  const at = `${where}.connect`;
  const obj = fields(raw, at, ['path', 'query', 'auth']);
  return {
    path: required(obj, 'path', at, isString, 'a string'),
    query: strMap(optional(obj, 'query', at, isObject, 'an object', {}), `${at}.query`),
    auth: 'auth' in obj ? oneOf(obj.auth, `${at}.auth`, ['none', 'bearer', 'query']) : 'none',
  };
}

function readExpect(raw: Record<string, unknown>, where: string): Expect | null {
  if (!('expect' in raw)) return null;
  const at = `${where}.expect`;
  const obj = fields(
    typed(raw.expect, at, isObject, 'an object'),
    at,
    ['status', 'headers', 'body', 'text'],
  );
  return {
    status: required(obj, 'status', at, isInt, 'an integer'),
    headers: typed(
      optional(obj, 'headers', at, isObject, 'an object', {}),
      `${at}.headers`,
      isObject,
      'an object',
    ),
    body: 'body' in obj ? obj.body : ABSENT,
    text: 'text' in obj ? obj.text : ABSENT,
  };
}
