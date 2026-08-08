/**
 * Tests for forge-egui's geometry, type and motion token generator.
 *
 * These assert relations between the source and the emitted Rust — that each
 * field carries its token's number in the unit egui wants, and that the field
 * names follow the token names. They never restate a token value: `just check`
 * already proves the committed file matches the source, so a literal here would
 * only be a second copy of it.
 */
import assert from 'node:assert/strict';
import test from 'node:test';

import { bannerLines } from './banner.mjs';
import { CONSTANTS, STRUCTS, measure, renderEguiTokens } from './egui-tokens.mjs';
import { SOURCE_PATH, inKit, tokenNamed, tokens } from '../../packages/tokens/tokens.source.mjs';

const rust = renderEguiTokens();

/** The number the source authored, with its unit dropped. */
const authored = (name) => Number.parseFloat(tokenNamed(name).value.raw);

/** The `Default` impl of a type, from its opening line to the closing brace. */
function defaultsOf(type) {
  const match = rust.match(new RegExp(`impl Default for ${type} \\{[\\s\\S]*?\\n\\}`));
  assert.ok(match, `no Default impl for ${type}`);
  return match[0];
}

/** The number a field is assigned in a `Default` impl. */
function assigned(type, field) {
  const match = defaultsOf(type).match(new RegExp(`\\b${field}: (-?[\\d.]+)`));
  assert.ok(match, `${type} does not assign ${field}`);
  return Number(match[1]);
}

/** Every token the emitted file reads, struct fields and shell constants alike. */
const read = [
  ...STRUCTS.flatMap((struct) => struct.fields.map(([, name]) => name)),
  ...CONSTANTS.map(([, name]) => name),
];

test('the file opens with a header naming it generated and pointing at the source', () => {
  const head = rust.slice(0, rust.indexOf('\n//!\n'));
  for (const line of bannerLines()) assert.ok(head.includes(line), `header is missing: ${line}`);
  assert.ok(head.includes(SOURCE_PATH));
});

test('every struct declares each of its fields and names the token behind it', () => {
  for (const { type, fields } of STRUCTS) {
    for (const [field, name] of fields) {
      assert.match(
        rust,
        new RegExp(`/// \`--${name}\`[^\\n]*\\n    pub ${field}: f32,`),
        `${type}.${field} does not declare --${name}`,
      );
    }
  }
});

test('a length reaches egui as points, one per pixel the source authored', () => {
  for (const { type, unit, fields } of STRUCTS.filter((s) => s.unit === 'points')) {
    for (const [field, name] of fields) {
      assert.equal(assigned(type, field), authored(name), `${type}.${field}`);
      assert.equal(measure(name, unit), authored(name));
    }
  }
});

test('a duration reaches egui as seconds, from the milliseconds the source authored', () => {
  const durations = STRUCTS.filter((s) => s.unit === 'seconds');
  assert.ok(durations.length, 'the layout has no durations to check');
  for (const { type, fields } of durations) {
    for (const [field, name] of fields) {
      assert.equal(assigned(type, field), authored(name) / 1000, `${type}.${field}`);
    }
  }
});

test('a shell constant carries its token, in points', () => {
  for (const [constant, name] of CONSTANTS) {
    const match = rust.match(
      new RegExp(`/// \`--${name}\`[^\\n]*\\npub const ${constant}: f32 = (-?[\\d.]+);`),
    );
    assert.ok(match, `${constant} does not carry --${name}`);
    assert.equal(Number(match[1]), authored(name), constant);
  }
});

/**
 * The Rust field a token name maps to: the name without its group prefix, and
 * with a leading digit moved to the end, because a Rust identifier cannot start
 * with one. This is the rule that keeps `--fs-xl` from being called `h3` again.
 */
const fieldFor = (name, prefix) => {
  const bare = name.slice(prefix.length);
  return /^\d/.test(bare) ? `${bare.slice(1)}${bare[0]}` : bare;
};

test('a field named after its token uses the token name, not a synonym', () => {
  const named = STRUCTS.filter((s) => s.prefix);
  assert.ok(named.length, 'no struct names its fields after its tokens');
  for (const { type, prefix, fields } of named) {
    for (const [field, name] of fields) {
      assert.ok(name.startsWith(prefix), `--${name} is not in the ${prefix}* group`);
      assert.equal(field, fieldFor(name, prefix), `${type}.${field} renames --${name}`);
    }
  }
});

test('the leading-digit rule reverses the name rather than inventing one', () => {
  assert.equal(fieldFor('fs-2xl', 'fs-'), 'xl2');
  assert.equal(fieldFor('fs-xl', 'fs-'), 'xl');
});

test('every token scoped to this kit is read; a scoped token nothing reads is dead', () => {
  const scoped = tokens.filter((t) => t.only?.includes('egui'));
  assert.ok(scoped.length, 'the source scopes nothing to egui');
  for (const token of scoped) assert.ok(read.includes(token.name), `nothing reads --${token.name}`);
});

test('no token fills two fields', () => {
  assert.equal(new Set(read).size, read.length);
});

test('a kit cannot read a token scoped to another kit', () => {
  const scoped = tokens.find((t) => t.only?.includes('egui') && !inKit(t, 'web'));
  assert.throws(() => tokenNamed(scoped.name, 'web'), /scoped to egui/);
  assert.doesNotThrow(() => tokenNamed(scoped.name, 'egui'));
});

test('a value authored in the wrong unit is refused, not silently rescaled', () => {
  // `--dur-1` is milliseconds; asking for it as a length must not yield 80pt.
  assert.throws(() => measure('dur-1', 'points'), /reads it as points/);
  assert.throws(() => measure('r-sm', 'seconds'), /reads it as seconds/);
});
