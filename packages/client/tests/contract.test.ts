/* Tests for the corpus reader and the matcher the driver runs on.

   These guard the driver's instruments. A matcher that says yes too easily,
   or a reader that skips a field it does not know, makes every case in
   `corpus.test.ts` weaker without failing anything. The same suite guards the
   Python reading in `python/forge-server/tests/test_contract.py`. */

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import {
  CORPUS_PATH,
  CorpusError,
  TS_CLIENT,
  casesFor,
  loadCorpus,
  parseCorpus,
} from './contract/corpus';
import { MatchError, interpolateValue, matchValue } from './contract/matcher';

const VARS = { user: 'admin' };

const AUTHORED = readFileSync(CORPUS_PATH, 'utf8');

function ok(expected: unknown, actual: unknown): void {
  matchValue(expected, actual, VARS);
}

function fails(expected: unknown, actual: unknown): string {
  try {
    matchValue(expected, actual, VARS);
  } catch (err) {
    if (err instanceof MatchError) return err.message;
    throw err;
  }
  throw new Error(`expected a MatchError for ${JSON.stringify(expected)}`);
}

function rejects(mutate: (corpus: any) => void, message: string | RegExp): void {
  const corpus = JSON.parse(AUTHORED);
  mutate(corpus);
  expect(() => parseCorpus(JSON.stringify(corpus))).toThrowError(
    expect.objectContaining({ constructor: CorpusError, message: expect.stringMatching(message) }),
  );
}

describe('the authored corpus', () => {
  it('is valid, and reaches this transport', () => {
    const corpus = loadCorpus();
    expect(corpus.contractVersion).toBe('1.0');
    expect(casesFor(corpus, TS_CLIENT).length).toBeGreaterThan(0);
  });

  it('rejects a case that ignores a transport', () => {
    // The rule that keeps gaps visible. Drop a transport from a case and the
    // corpus must refuse to load.
    rejects((corpus) => {
      corpus.cases[0].applies = ['rust-http'];
      corpus.cases[0].inapplicable = {};
    }, /says nothing about transport 'python-http'/);
  });

  it('rejects an excuse with no reason', () => {
    rejects((corpus) => {
      const c = corpus.cases.find((x: any) => Object.keys(x.inapplicable ?? {}).length > 0);
      for (const transport of Object.keys(c.inapplicable)) c.inapplicable[transport] = '  ';
    }, /with no reason/);
  });

  it('rejects a transport that both applies and is excused', () => {
    rejects((corpus) => {
      corpus.cases[0].inapplicable = { [TS_CLIENT]: 'cannot' };
    }, /both applies to and excuses/);
  });

  it('rejects duplicate case ids', () => {
    rejects((corpus) => {
      corpus.cases.push(corpus.cases[0]);
    }, /duplicate case id/);
  });

  it('does not read past an unknown field', () => {
    // A field the reader does not know is a typo, and a typo in an
    // expectation is an assertion that never runs.
    rejects((corpus) => {
      corpus.cases[0].steps[0].expect.bodyy = { ok: true };
    }, /unknown field/);
  });

  it('rejects a step it does not know', () => {
    rejects((corpus) => {
      corpus.cases[0].steps.push({ await_nothing: {} });
    }, /not a step/);
  });

  it('rejects a stream case whose fixture mounts no event bus', () => {
    rejects((corpus) => {
      const c = corpus.cases.find((x: any) => x.kind === 'ws');
      delete corpus.fixtures[c.fixture ?? 'default'].events;
    }, /mounts no event bus/);
  });

  it('rejects a body expectation on the request that opens a stream', () => {
    rejects((corpus) => {
      const c = corpus.cases.find((x: any) => x.kind === 'sse');
      c.steps[0].expect.body = { ok: true };
    }, /expects a body from the request/);
  });
});

describe('the matcher', () => {
  it('matches objects by subset', () => {
    ok({ ok: true }, { ok: true, data: 1 });
    expect(fails({ ok: true }, { data: 1 })).toContain('$.ok: missing');
  });

  it('rejects extra keys under $exact', () => {
    ok({ $exact: { n: 1 } }, { n: 1 });
    expect(fails({ $exact: { n: 1 } }, { n: 1, x: 2 })).toContain('unexpected');
  });

  it('matches arrays element-wise and by length', () => {
    ok([{ a: 1 }], [{ a: 1, b: 2 }]);
    expect(fails([1], [1, 2])).toContain('expected 1 elements, got 2');
  });

  it('reads $contains over strings and arrays', () => {
    ok({ $contains: 'echo' }, ['echo', 'publish']);
    ok({ $contains: 'echo' }, 'no action echo here');
    ok({ $contains: { name: 'b' } }, [{ name: 'a' }, { name: 'b' }]);
    expect(fails({ $contains: 'gone' }, ['echo'])).toContain('no element');
  });

  it('checks types and number bounds', () => {
    ok({ $type: 'integer' }, 3);
    ok({ $gt: 0 }, 1.5);
    expect(fails({ $type: 'integer' }, 1.5)).toContain('expected a integer');
    expect(fails({ $gt: 0 }, 0)).toContain('not greater');
  });

  it('rejects the empty string under $min_length', () => {
    // `$type: "string"` would accept "", which is what a token must never be.
    ok({ $min_length: 1 }, 'a');
    expect(fails({ $min_length: 1 }, '')).toContain('shorter than 1');
    expect(fails({ $min_length: 1 }, 7)).toContain('needs a string');
  });

  it('interpolates strings before comparing', () => {
    ok('${user}', 'admin');
    expect(fails('${user}', 'ops')).toContain('expected "admin"');
    expect(fails('${nope}', 'x')).toContain('unknown variable');
  });

  it.each([
    [{ $min_length: 'one' }, '$min_length takes a count'],
    [{ $type: 7 }, '$type takes a type name'],
    [{ $type: 'strng' }, "unknown $type 'strng'"],
    [{ $prefix: 7 }, '$prefix takes a string'],
    [{ $gt: '0' }, '$gt takes a number'],
    [{ $nope: 1 }, "unknown matcher '$nope'"],
  ])('reads a wrong operand as a corpus bug: %j', (expected, complaint) => {
    // An operand the matcher cannot use is a corpus bug, and must read as one
    // rather than as whatever error using it happens to raise.
    const error = fails(expected, 'anything');
    expect(error).toContain(complaint);
    expect(error.startsWith('at $:')).toBe(true);
  });

  it('reads a mixed operator object as a corpus bug', () => {
    expect(fails({ $type: 'object', n: 1 }, { n: 1 })).toContain('exactly one $ key');
  });

  it('interpolates keys and nested values', () => {
    expect(interpolateValue({ '${user}': ['${user}', 1] }, VARS)).toEqual({ admin: ['admin', 1] });
  });

  it('points the error path at the field', () => {
    const error = fails({ data: { sub: '${user}' } }, { data: { sub: 'ops' } });
    expect(error.startsWith('at $.data.sub:')).toBe(true);
  });
});
