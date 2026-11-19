export interface KeyPress {
  readonly code: string;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
  readonly metaKey: boolean;
}

type ModifierFlag = 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey';

const MODIFIERS: readonly (readonly [ModifierFlag, string])[] = [
  ['ctrlKey', 'Ctrl'],
  ['altKey', 'Alt'],
  ['shiftKey', 'Shift'],
  ['metaKey', 'Win'],
];

const MODIFIER_CODE = /^(?:Control|Alt|Shift|Meta|OS)(?:Left|Right)$/;
const LETTER_OR_DIGIT = /^(?:Key|Digit)([A-Z0-9])$/;

export const CANCEL_CODE = 'Escape';

export function keyName(code: string): string | null {
  if (code === '' || MODIFIER_CODE.test(code)) {
    return null;
  }
  return LETTER_OR_DIGIT.exec(code)?.[1] ?? code;
}

export function chordOf(press: KeyPress): string | null {
  const key = keyName(press.code);
  if (key === null) {
    return null;
  }
  const held = MODIFIERS.filter(([flag]) => press[flag]).map(([, label]) => label);
  return [...held, key].join('+');
}
