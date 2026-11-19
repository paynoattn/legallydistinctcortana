import { describe, expect, it } from 'vitest';
import {
  anyOf,
  arrayOf,
  integerIn,
  isBoolean,
  isString,
  nullable,
  objectOf,
  oneOf,
  parseJson,
} from './checks.ts';

describe('isString', () => {
  it.each(['', 'hello'])('accepts %j', (value) => {
    expect(isString(value)).toBe(true);
  });

  it.each([null, undefined, true, false, 0, [], {}])('rejects %j', (value) => {
    expect(isString(value)).toBe(false);
  });
});

describe('isBoolean', () => {
  it.each([true, false])('accepts %j', (value) => {
    expect(isBoolean(value)).toBe(true);
  });

  it.each([null, undefined, '', 'true', 'false', 0, 1, [], {}])('rejects %j', (value) => {
    expect(isBoolean(value)).toBe(false);
  });
});

describe('integerIn', () => {
  it.each([-2, -1, 0, 1, 2])('accepts the inclusive range value %j', (value) => {
    expect(integerIn(-2, 2)(value)).toBe(true);
  });

  it.each([
    -3,
    3,
    -1.5,
    0.5,
    Number.NaN,
    Number.POSITIVE_INFINITY,
    Number.NEGATIVE_INFINITY,
    '1',
    '',
    true,
    false,
    null,
    undefined,
    [],
    [1],
    {},
    1n,
    Symbol('number'),
  ])('rejects %s', (value) => {
    expect(integerIn(-2, 2)(value)).toBe(false);
  });

  it.each([
    [0, true],
    [-1, false],
    [1, false],
  ])('checks %j against a single-value range', (value, accepted) => {
    expect(integerIn(0, 0)(value)).toBe(accepted);
  });

  it.each([-1, 0, 1])('rejects %j for an inverted range', (value) => {
    expect(integerIn(1, -1)(value)).toBe(false);
  });
});

describe('oneOf', () => {
  it.each(['first', 'second', 'third'])('accepts the member %j', (value) => {
    expect(oneOf('first', 'second', 'third')(value)).toBe(true);
  });

  it.each(['', 'FIRST', 'other', null, undefined, 0, {}, []])('rejects %j', (value) => {
    expect(oneOf('first', 'second', 'third')(value)).toBe(false);
  });

  it.each([null, undefined, '', 0])('accepts nothing with no options, including %j', (value) => {
    expect(oneOf()(value)).toBe(false);
  });

  it('compares objects by identity', () => {
    const member = { name: 'member' };
    const check = oneOf(member);
    expect(check(member)).toBe(true);
    expect(check({ name: 'member' })).toBe(false);
  });

  it.each([null, undefined, true, 1, Number.NaN])('accepts a non-string member %j', (value) => {
    expect(oneOf(null, undefined, true, 1, Number.NaN)(value)).toBe(true);
  });
});

describe('nullable', () => {
  it.each([null, '', 'hello'])('accepts %j', (value) => {
    expect(nullable(isString)(value)).toBe(true);
  });

  it.each([undefined, false, 0, [], {}])('rejects %j', (value) => {
    expect(nullable(isString)(value)).toBe(false);
  });
});

describe('arrayOf', () => {
  it.each([{ value: [] }, { value: [''] }, { value: ['first', 'second'] }])(
    'accepts $value',
    ({ value }) => {
      expect(arrayOf(isString)(value)).toBe(true);
    },
  );

  it.each([null, undefined, '', 'hello', 0, false, {}, { length: 0 }])('rejects %j', (value) => {
    expect(arrayOf(isString)(value)).toBe(false);
  });

  it.each([{ value: [1] }, { value: ['ok', 1] }, { value: [1, 'ok'] }])(
    'rejects a wrong-typed array $value',
    ({ value }) => {
      expect(arrayOf(isString)(value)).toBe(false);
    },
  );
});

describe('objectOf', () => {
  const check = objectOf({ name: isString, enabled: isBoolean });

  it.each([
    { name: 'hello', enabled: true },
    { name: '', enabled: false },
    { name: 'hello', enabled: true, extra: 1 },
  ])('accepts %j', (value) => {
    expect(check(value)).toBe(true);
  });

  it.each([
    null,
    undefined,
    '',
    'hello',
    0,
    false,
    [],
    {},
    { name: 'hello' },
    { enabled: true },
    { name: undefined, enabled: true },
    { name: 'hello', enabled: undefined },
    { name: 0, enabled: true },
    { name: 'hello', enabled: 'true' },
  ])('rejects %j', (value) => {
    expect(check(value)).toBe(false);
  });

  it('accepts an empty object for an empty shape', () => {
    expect(objectOf({})({})).toBe(true);
  });
});

describe('anyOf', () => {
  it.each(['', 'hello', true, false])('accepts a match from either checker: %j', (value) => {
    expect(anyOf(isString, isBoolean)(value)).toBe(true);
  });

  it.each([null, undefined, 0, [], {}])('rejects %j when neither checker matches', (value) => {
    expect(anyOf(isString, isBoolean)(value)).toBe(false);
  });

  it.each(['hello', true, null, undefined])('rejects %j when there are no checkers', (value) => {
    expect(anyOf()(value)).toBe(false);
  });

  it('accepts a value matching both checkers', () => {
    expect(anyOf(isString, oneOf('hello'))('hello')).toBe(true);
  });

  it('delegates to a single checker', () => {
    const check = anyOf(isBoolean);
    expect(check(false)).toBe(true);
    expect(check('false')).toBe(false);
  });
});

describe('parseJson', () => {
  it.each([
    { raw: '{"name":"hello","enabled":false}', expected: { name: 'hello', enabled: false } },
    { raw: '[1,"hello",null]', expected: [1, 'hello', null] },
    { raw: '"hello"', expected: 'hello' },
    { raw: '""', expected: '' },
    { raw: '0', expected: 0 },
    { raw: 'true', expected: true },
    { raw: 'false', expected: false },
    { raw: 'null', expected: null },
    { raw: '  { }  ', expected: {} },
  ])('parses $raw', ({ raw, expected }) => {
    expect(parseJson(raw)).toEqual(expected);
  });

  it.each([null, '', ' ', 'undefined', 'not json', '{', '[1,]', '{"name":}', 'NaN'])(
    'returns undefined for %j',
    (raw) => {
      expect(parseJson(raw)).toBeUndefined();
    },
  );
});
