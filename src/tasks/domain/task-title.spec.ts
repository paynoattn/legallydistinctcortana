import { describe, expect, it } from 'vitest';
import { isTaskTitle, MAX_TITLE_LENGTH, parseTaskTitle } from './task-title.ts';

describe('parseTaskTitle', () => {
  it('trims surrounding whitespace', () => {
    expect(parseTaskTitle('  Buy milk  ')).toEqual({ ok: true, title: 'Buy milk' });
  });

  it.each([
    [0, { ok: false, error: 'empty' }],
    [1, { ok: true, title: 'a' }],
    [MAX_TITLE_LENGTH, { ok: true, title: 'a'.repeat(MAX_TITLE_LENGTH) }],
    [MAX_TITLE_LENGTH + 1, { ok: false, error: 'too-long' }],
  ])('a title of %i characters gives %o', (length, expected) => {
    expect(parseTaskTitle('a'.repeat(length))).toEqual(expected);
  });

  it('rejects a whitespace-only title as empty', () => {
    expect(parseTaskTitle('   ')).toEqual({ ok: false, error: 'empty' });
  });

  it('measures length after trimming', () => {
    const title = 'a'.repeat(MAX_TITLE_LENGTH);
    expect(parseTaskTitle(` ${title} `)).toEqual({ ok: true, title });
  });
});

describe('isTaskTitle', () => {
  it.each([
    ['a', true],
    ['', false],
    [' a', false],
    ['a ', false],
    ['a'.repeat(MAX_TITLE_LENGTH), true],
    ['a'.repeat(MAX_TITLE_LENGTH + 1), false],
    [1, false],
    [null, false],
  ])('%o is a title: %s', (value, expected) => {
    expect(isTaskTitle(value)).toBe(expected);
  });
});
