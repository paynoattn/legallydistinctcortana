import { wordAt } from './word-timing.ts';

export type ReaderStatus =
  | { readonly kind: 'idle' }
  | { readonly kind: 'reading' }
  | { readonly kind: 'paused' }
  | { readonly kind: 'finished' }
  | { readonly kind: 'failed'; readonly message: string };

export interface SentenceTiming {
  readonly startAt: number;
  readonly starts: readonly number[];
}

export interface Position {
  readonly sentence: number;
  readonly word: number | null;
}

const RESTART_AFTER_MS = 1500;
export const DEFAULT_TEXT_SIZE = 25;
const MIN_TEXT_SIZE = 20;
const MAX_TEXT_SIZE = 40;
const TEXT_SIZE_STEP = 3;

function latestStarted(
  timings: ReadonlyMap<number, SentenceTiming>,
  now: number,
): readonly [number, SentenceTiming] | undefined {
  return [...timings]
    .filter(([, timing]) => timing.startAt <= now)
    .toSorted(([first], [second]) => second - first)[0];
}

export function positionAt(
  timings: ReadonlyMap<number, SentenceTiming>,
  from: number,
  now: number,
): Position {
  const latest = latestStarted(timings, now);
  if (latest === undefined) {
    return { sentence: from, word: null };
  }
  const [sentence, timing] = latest;
  return { sentence, word: wordAt(timing.starts, now - timing.startAt) };
}

export function previousSentence(current: number, playedMs: number): number {
  return playedMs > RESTART_AFTER_MS ? current : Math.max(0, current - 1);
}

export function resizedText(size: number, direction: 1 | -1): number {
  return Math.min(MAX_TEXT_SIZE, Math.max(MIN_TEXT_SIZE, size + direction * TEXT_SIZE_STEP));
}

export function statusLine(status: ReaderStatus, sentence: number, count: number): string {
  const place = `Sentence ${String(sentence + 1)} of ${String(count)}.`;
  switch (status.kind) {
    case 'idle':
      return '';
    case 'reading':
      return place;
    case 'paused':
      return `Paused. ${place}`;
    case 'finished':
      return 'Finished reading.';
    case 'failed':
      return status.message;
  }
}

export function storedTextSize(stored: string | null): number {
  const size = Number(stored ?? Number.NaN);
  return Number.isInteger(size) && MIN_TEXT_SIZE <= size && size <= MAX_TEXT_SIZE
    ? size
    : DEFAULT_TEXT_SIZE;
}
