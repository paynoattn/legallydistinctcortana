import { describe, expect, it } from 'vitest';
import {
  DEFAULT_TEXT_SIZE,
  positionAt,
  previousSentence,
  resizedText,
  type SentenceTiming,
  statusLine,
  storedTextSize,
} from './playback.ts';

function timings(
  entries: readonly (readonly [number, SentenceTiming])[],
): ReadonlyMap<number, SentenceTiming> {
  return new Map(entries);
}

describe('positionAt', () => {
  const second: SentenceTiming = { startAt: 1000, starts: [0, 250, 700] };
  const third: SentenceTiming = { startAt: 2000, starts: [0, 400] };

  it('stays on the sentence the reading started from until one is heard', () => {
    expect(positionAt(timings([]), 2, 0)).toEqual({ sentence: 2, word: null });
    expect(positionAt(timings([[1, second]]), 1, 999)).toEqual({ sentence: 1, word: null });
  });

  it('lights the word being spoken in the latest sentence that has started', () => {
    expect(positionAt(timings([[1, second]]), 1, 1000)).toEqual({ sentence: 1, word: 0 });
    expect(
      positionAt(
        timings([
          [1, second],
          [2, third],
        ]),
        1,
        1700,
      ),
    ).toEqual({
      sentence: 1,
      word: 2,
    });
    expect(
      positionAt(
        timings([
          [2, third],
          [1, second],
        ]),
        1,
        2400,
      ),
    ).toEqual({
      sentence: 2,
      word: 1,
    });
    expect(
      positionAt(
        timings([
          [1, second],
          [2, third],
        ]),
        1,
        2400,
      ),
    ).toEqual({
      sentence: 2,
      word: 1,
    });
  });

  it('keeps the last word lit in the pause after a sentence', () => {
    expect(positionAt(timings([[1, second]]), 1, 9000)).toEqual({ sentence: 1, word: 2 });
  });
});

describe('previousSentence', () => {
  it.each([
    [2, 0, 1],
    [2, 1500, 1],
    [2, 1501, 2],
    [0, 0, 0],
    [0, 3000, 0],
  ])('from sentence %i after %i ms goes to %i', (current, played, expected) => {
    expect(previousSentence(current, played)).toBe(expected);
  });
});

describe('resizedText', () => {
  it('starts at 25 px and moves in 3 px steps between 20 and 40 px', () => {
    expect(DEFAULT_TEXT_SIZE).toBe(25);
    expect(resizedText(25, 1)).toBe(28);
    expect(resizedText(25, -1)).toBe(22);
    expect(resizedText(37, 1)).toBe(40);
    expect(resizedText(40, 1)).toBe(40);
    expect(resizedText(22, -1)).toBe(20);
    expect(resizedText(20, -1)).toBe(20);
  });
});

describe('statusLine', () => {
  it.each([
    [{ kind: 'idle' } as const, ''],
    [{ kind: 'reading' } as const, 'Sentence 3 of 12.'],
    [{ kind: 'paused' } as const, 'Paused. Sentence 3 of 12.'],
    [{ kind: 'finished' } as const, 'Finished reading.'],
    [{ kind: 'failed', message: 'no voice' } as const, 'no voice'],
  ])('says where the reading is when %j', (status, line) => {
    expect(statusLine(status, 2, 12)).toBe(line);
  });
});

describe('storedTextSize', () => {
  it.each([
    ['20', 20],
    ['31', 31],
    ['40', 40],
  ])('keeps the stored size %j', (stored, size) => {
    expect(storedTextSize(stored)).toBe(size);
  });

  it.each([null, '19', '41', '25.5', 'big', ''])('falls back to the default for %j', (stored) => {
    expect(storedTextSize(stored)).toBe(DEFAULT_TEXT_SIZE);
  });
});
