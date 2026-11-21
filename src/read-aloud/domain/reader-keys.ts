import { type MoodId, moodForKey } from './mood.ts';

export type ReaderCommand =
  | { readonly kind: 'toggle' | 'previous' | 'next' | 'larger' | 'smaller' | 'close' }
  | { readonly kind: 'mood'; readonly mood: MoodId };

export interface KeyPress {
  readonly key: string;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly metaKey: boolean;
}

const PLAIN: ReadonlyMap<string, ReaderCommand> = new Map([
  [' ', { kind: 'toggle' }],
  ['ArrowLeft', { kind: 'previous' }],
  ['ArrowRight', { kind: 'next' }],
  ['Escape', { kind: 'close' }],
]);

const WITH_CTRL: ReadonlyMap<string, ReaderCommand> = new Map([
  ['+', { kind: 'larger' }],
  ['=', { kind: 'larger' }],
  ['-', { kind: 'smaller' }],
]);

export function commandFor(press: KeyPress): ReaderCommand | undefined {
  if (press.altKey || press.metaKey) {
    return undefined;
  }
  if (press.ctrlKey) {
    return WITH_CTRL.get(press.key);
  }
  const mood = moodForKey(press.key);
  return mood === undefined ? PLAIN.get(press.key) : { kind: 'mood', mood: mood.id };
}
