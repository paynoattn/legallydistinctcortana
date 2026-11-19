import { describe, expect, it } from 'vitest';
import { CANCEL_CODE, chordOf, type KeyPress, keyName } from './chord.ts';

function press(code: string, held: Partial<Omit<KeyPress, 'code'>> = {}): KeyPress {
  return { code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...held };
}

describe('keyName', () => {
  it.each([
    ['KeyA', 'A'],
    ['KeyZ', 'Z'],
    ['Digit0', '0'],
    ['Digit9', '9'],
    ['F1', 'F1'],
    ['F24', 'F24'],
    ['Space', 'Space'],
    ['Enter', 'Enter'],
    ['Backquote', 'Backquote'],
    ['Minus', 'Minus'],
    ['Numpad5', 'Numpad5'],
    ['ArrowUp', 'ArrowUp'],
    ['Insert', 'Insert'],
    ['PageUp', 'PageUp'],
    ['KeyAB', 'KeyAB'],
    ['XKeyA', 'XKeyA'],
    ['Digit10', 'Digit10'],
  ])('names %s as %s', (code, name) => {
    expect(keyName(code)).toBe(name);
  });

  it.each([
    'ControlLeft',
    'ControlRight',
    'AltLeft',
    'AltRight',
    'ShiftLeft',
    'ShiftRight',
    'MetaLeft',
    'MetaRight',
    'OSLeft',
    'OSRight',
    '',
  ])('treats %s as no key', (code) => {
    expect(keyName(code)).toBeNull();
  });

  it.each(['ControlLeftX', 'XControlLeft', 'Control', 'Left'])('keeps %s as a key', (code) => {
    expect(keyName(code)).toBe(code);
  });
});

describe('chordOf', () => {
  it('names a bare key', () => {
    expect(chordOf(press('F9'))).toBe('F9');
  });

  it('puts the modifiers first, in the order Ctrl, Alt, Shift, Win', () => {
    expect(
      chordOf(press('KeyR', { metaKey: true, shiftKey: true, altKey: true, ctrlKey: true })),
    ).toBe('Ctrl+Alt+Shift+Win+R');
  });

  it.each([
    [{ ctrlKey: true }, 'Ctrl+Space'],
    [{ altKey: true }, 'Alt+Space'],
    [{ shiftKey: true }, 'Shift+Space'],
    [{ metaKey: true }, 'Win+Space'],
    [{ ctrlKey: true, shiftKey: true }, 'Ctrl+Shift+Space'],
  ])('names a chord with %o', (held, chord) => {
    expect(chordOf(press('Space', held))).toBe(chord);
  });

  it('waits for a key when only modifiers are held', () => {
    expect(chordOf(press('ControlLeft', { ctrlKey: true }))).toBeNull();
  });

  it('cancels on Escape', () => {
    expect(CANCEL_CODE).toBe('Escape');
  });
});
