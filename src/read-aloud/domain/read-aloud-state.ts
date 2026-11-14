import type { MoodId } from './mood.ts';

export type Selection =
  | { readonly kind: 'waiting' }
  | { readonly kind: 'ready'; readonly preview: string; readonly charCount: number }
  | { readonly kind: 'failed'; readonly message: string };

export type ReadingStatus =
  | { readonly kind: 'idle' }
  | { readonly kind: 'reading'; readonly mood: MoodId }
  | { readonly kind: 'finished' }
  | { readonly kind: 'stopped' }
  | { readonly kind: 'failed'; readonly message: string };

export const WAITING: Selection = { kind: 'waiting' };
export const IDLE: ReadingStatus = { kind: 'idle' };

export function describeLength(charCount: number): string {
  return charCount === 1 ? '1 character' : `${String(charCount)} characters`;
}
