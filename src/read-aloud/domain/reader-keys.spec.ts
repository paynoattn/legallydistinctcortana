import { describe, expect, it } from 'vitest';
import { commandFor, type KeyPress } from './reader-keys.ts';

function press(key: string, modifiers: Partial<Omit<KeyPress, 'key'>> = {}): KeyPress {
  return { key, ctrlKey: false, altKey: false, metaKey: false, ...modifiers };
}

describe('commandFor', () => {
  it.each([
    [' ', 'toggle'],
    ['ArrowLeft', 'previous'],
    ['ArrowRight', 'next'],
    ['Escape', 'close'],
  ])('maps %j to %s', (key, kind) => {
    expect(commandFor(press(key))).toEqual({ kind });
  });

  it.each([
    ['1', 'neutral'],
    ['5', 'urgent'],
    ['9', 'deadpan'],
  ])('picks a mood with %s', (key, mood) => {
    expect(commandFor(press(key))).toEqual({ kind: 'mood', mood });
  });

  it.each([
    ['+', 'larger'],
    ['=', 'larger'],
    ['-', 'smaller'],
  ])('sizes the text with Ctrl and %s', (key, kind) => {
    expect(commandFor(press(key, { ctrlKey: true }))).toEqual({ kind });
  });

  it.each(['+', '=', '-'])('does not size the text with %s alone', (key) => {
    expect(commandFor(press(key))).toBeUndefined();
  });

  it.each([
    press('1', { ctrlKey: true }),
    press(' ', { ctrlKey: true }),
    press('ArrowLeft', { altKey: true }),
    press('5', { metaKey: true }),
    press('+', { ctrlKey: true, altKey: true }),
    press('0'),
    press('Enter'),
    press('a'),
  ])('ignores %j', (key) => {
    expect(commandFor(key)).toBeUndefined();
  });
});
