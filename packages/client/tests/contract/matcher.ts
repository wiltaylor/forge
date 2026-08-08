/* `${name}` substitution and the matcher the expectations are written in.

   Objects match by subset, so a case asserts the fields it is about and stays
   quiet about the rest; `$exact` opts back into deep equality. See
   `contract/README.md` for the full table.

   This is the TypeScript reading of the same rules `crates/forge-contract`
   reads in Rust and `python/forge-server/tests/contract` reads in Python. The
   corpus is the shared thing; each language brings its own matcher. */

export type Vars = Record<string, string>;

/** An expectation the response did not meet, or a corpus bug. */
export class MatchError extends Error {}

/** Replace every `${name}` in `text`.

    An unknown name is an error rather than an empty string: a typo in the
    corpus should fail loudly. */
export function interpolate(text: string, vars: Vars): string {
  const out: string[] = [];
  let rest = text;
  for (;;) {
    const start = rest.indexOf('${');
    if (start < 0) {
      out.push(rest);
      return out.join('');
    }
    out.push(rest.slice(0, start));
    const tail = rest.slice(start + 2);
    const end = tail.indexOf('}');
    if (end < 0) throw new MatchError(`unterminated \${ in ${JSON.stringify(text)}`);
    const name = tail.slice(0, end);
    const value = vars[name];
    if (value === undefined) {
      throw new MatchError(`unknown variable \${${name}} in ${JSON.stringify(text)}`);
    }
    out.push(value);
    rest = tail.slice(end + 1);
  }
}

/** Interpolate every string in a JSON value, keys included. */
export function interpolateValue(value: unknown, vars: Vars): unknown {
  if (typeof value === 'string') return interpolate(value, vars);
  if (Array.isArray(value)) return value.map((item) => interpolateValue(item, vars));
  if (isObject(value)) {
    const out: Record<string, unknown> = {};
    for (const [key, item] of Object.entries(value)) {
      out[interpolate(key, vars)] = interpolateValue(item, vars);
    }
    return out;
  }
  return value;
}

/** Check `actual` against the authored matcher `expected`.

    Throws a `MatchError` naming the path it failed at, so a mismatch deep in
    an envelope points at the field rather than at the whole body. */
export function matchValue(expected: unknown, actual: unknown, vars: Vars): void {
  check(expected, actual, vars, '$');
}

/** A plain JSON object: not null, not an array. */
export function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function check(expected: unknown, actual: unknown, vars: Vars, path: string): void {
  if (isObject(expected)) {
    const operator = operatorOf(expected, path);
    if (operator === null) checkSubset(expected, actual, vars, path);
    else checkOperator(operator[0], operator[1], actual, vars, path);
  } else if (Array.isArray(expected)) {
    checkArray(expected, actual, vars, path);
  } else if (typeof expected === 'string') {
    equal(interpolate(expected, vars), actual, path);
  } else {
    equal(expected, actual, path);
  }
}

/** An operator object holds exactly one `$` key and nothing else.

    A mix of the two is a corpus bug that would otherwise pass silently. */
function operatorOf(expected: Record<string, unknown>, path: string): [string, unknown] | null {
  const keys = Object.keys(expected);
  const dollars = keys.filter((key) => key.startsWith('$'));
  if (dollars.length === 0) return null;
  if (dollars.length !== keys.length || keys.length !== 1) {
    throw new MatchError(
      `at ${path}: an operator object holds exactly one $ key, found ${JSON.stringify(keys.sort())}`,
    );
  }
  const key = dollars[0] as string;
  return [key, expected[key]];
}

function checkOperator(
  operator: string,
  operand: unknown,
  actual: unknown,
  vars: Vars,
  path: string,
): void {
  if (operator === '$exact') {
    checkExact(operand, actual, vars, path);
  } else if (operator === '$min_length') {
    const wanted = intOperand(operand, '$min_length takes a count', path);
    if (typeof actual !== 'string') {
      throw new MatchError(`at ${path}: $min_length needs a string, got ${show(actual)}`);
    }
    if (actual.length < wanted) {
      throw new MatchError(
        `at ${path}: ${JSON.stringify(actual)} is shorter than ${wanted} characters`,
      );
    }
  } else if (operator === '$type') {
    if (typeof operand !== 'string') {
      throw new MatchError(`at ${path}: $type takes a type name, got ${show(operand)}`);
    }
    const accepts = TYPES[operand];
    if (accepts === undefined) {
      throw new MatchError(`at ${path}: unknown $type '${operand}'`);
    }
    if (!accepts(actual)) {
      throw new MatchError(`at ${path}: expected a ${operand}, got ${show(actual)}`);
    }
  } else if (operator === '$contains') {
    checkContains(operand, actual, vars, path);
  } else if (operator === '$prefix') {
    if (typeof operand !== 'string') {
      throw new MatchError(`at ${path}: $prefix takes a string, got ${show(operand)}`);
    }
    const wanted = interpolate(operand, vars);
    if (typeof actual !== 'string') {
      throw new MatchError(`at ${path}: $prefix needs a string, got ${show(actual)}`);
    }
    if (!actual.startsWith(wanted)) {
      throw new MatchError(
        `at ${path}: ${JSON.stringify(actual)} does not start with ${JSON.stringify(wanted)}`,
      );
    }
  } else if (operator === '$gt') {
    if (typeof operand !== 'number') {
      throw new MatchError(`at ${path}: $gt takes a number, got ${show(operand)}`);
    }
    if (typeof actual !== 'number') {
      throw new MatchError(`at ${path}: $gt needs a number, got ${show(actual)}`);
    }
    if (!(actual > operand)) {
      throw new MatchError(`at ${path}: ${actual} is not greater than ${operand}`);
    }
  } else {
    throw new MatchError(`at ${path}: unknown matcher '${operator}'`);
  }
}

/** An operator's own argument, checked before it is used.

    A corpus that authors `{"$min_length": "one"}` is a corpus bug, and must
    read as one rather than as whatever error using it happens to raise. */
function intOperand(operand: unknown, complaint: string, path: string): number {
  if (typeof operand !== 'number' || !Number.isInteger(operand)) {
    throw new MatchError(`at ${path}: ${complaint}, got ${show(operand)}`);
  }
  return operand;
}

function checkContains(operand: unknown, actual: unknown, vars: Vars, path: string): void {
  if (typeof actual === 'string') {
    if (typeof operand !== 'string') {
      throw new MatchError(`at ${path}: $contains on a string takes a string`);
    }
    const needle = interpolate(operand, vars);
    if (!actual.includes(needle)) {
      throw new MatchError(
        `at ${path}: ${JSON.stringify(actual)} does not contain ${JSON.stringify(needle)}`,
      );
    }
    return;
  }
  if (Array.isArray(actual)) {
    for (const item of actual) {
      try {
        check(operand, item, vars, path);
        return;
      } catch (err) {
        if (!(err instanceof MatchError)) throw err;
      }
    }
    throw new MatchError(`at ${path}: no element matches ${show(operand)}, got ${show(actual)}`);
  }
  throw new MatchError(`at ${path}: $contains needs a string or an array, got ${show(actual)}`);
}

/** The type names `$type` takes, and what each one accepts. JSON has one
    number type; `integer` is the whole-number subset of it, which is as fine
    a reading as JavaScript can give. */
const TYPES: Record<string, (value: unknown) => boolean> = {
  string: (value) => typeof value === 'string',
  number: (value) => typeof value === 'number',
  integer: (value) => typeof value === 'number' && Number.isInteger(value),
  boolean: (value) => typeof value === 'boolean',
  array: (value) => Array.isArray(value),
  object: (value) => isObject(value),
  null: (value) => value === null,
};

/** `$exact`: no extra keys, anywhere below this point.

    Matchers still apply, so a payload whose shape is the point can still say
    `{"$type": "integer"}` for the one field that moves. */
function checkExact(expected: unknown, actual: unknown, vars: Vars, path: string): void {
  if (isObject(expected)) {
    const operator = operatorOf(expected, path);
    if (operator !== null) {
      checkOperator(operator[0], operator[1], actual, vars, path);
      return;
    }
    if (!isObject(actual)) {
      throw new MatchError(`at ${path}: expected an object, got ${show(actual)}`);
    }
    const wanted = new Map<string, unknown>();
    for (const [key, want] of Object.entries(expected)) {
      wanted.set(interpolate(key, vars), want);
    }
    for (const key of Object.keys(actual)) {
      if (!wanted.has(key)) throw new MatchError(`at ${path}.${key}: unexpected`);
    }
    for (const [key, want] of wanted) {
      const child = `${path}.${key}`;
      if (!(key in actual)) throw new MatchError(`at ${child}: missing`);
      checkExact(want, actual[key], vars, child);
    }
  } else if (Array.isArray(expected)) {
    if (!Array.isArray(actual)) {
      throw new MatchError(`at ${path}: expected an array, got ${show(actual)}`);
    }
    if (expected.length !== actual.length) {
      throw new MatchError(
        `at ${path}: expected ${expected.length} elements, got ${actual.length}`,
      );
    }
    expected.forEach((want, index) => checkExact(want, actual[index], vars, `${path}[${index}]`));
  } else if (typeof expected === 'string') {
    equal(interpolate(expected, vars), actual, path);
  } else {
    equal(expected, actual, path);
  }
}

function checkSubset(
  expected: Record<string, unknown>,
  actual: unknown,
  vars: Vars,
  path: string,
): void {
  if (!isObject(actual)) {
    throw new MatchError(`at ${path}: expected an object, got ${show(actual)}`);
  }
  for (const [rawKey, want] of Object.entries(expected)) {
    const key = interpolate(rawKey, vars);
    const child = `${path}.${key}`;
    if (!(key in actual)) throw new MatchError(`at ${child}: missing`);
    check(want, actual[key], vars, child);
  }
}

function checkArray(expected: unknown[], actual: unknown, vars: Vars, path: string): void {
  if (!Array.isArray(actual)) {
    throw new MatchError(`at ${path}: expected an array, got ${show(actual)}`);
  }
  if (expected.length !== actual.length) {
    throw new MatchError(`at ${path}: expected ${expected.length} elements, got ${actual.length}`);
  }
  expected.forEach((want, index) => check(want, actual[index], vars, `${path}[${index}]`));
}

function equal(expected: unknown, actual: unknown, path: string): void {
  if (expected !== actual) {
    throw new MatchError(`at ${path}: expected ${show(expected)}, got ${show(actual)}`);
  }
}

function show(value: unknown): string {
  const text = JSON.stringify(value);
  return text === undefined ? String(value) : text;
}
