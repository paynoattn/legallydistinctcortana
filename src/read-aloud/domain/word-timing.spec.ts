import { describe, expect, it } from 'vitest';
import { wordAt, wordStarts, wordWeight } from './word-timing.ts';

describe('wordWeight', () => {
  it.each([
    ['Two', 1.25],
    ['emails', 2.25],
    ['morning.', 2.25],
    ['you.', 1.25],
    ['make', 1.25],
    ['table', 2.25],
    ['the', 1.25],
    ['rhythm', 1.25],
    ['open', 2.25],
    ['middleware', 3.25],
    ['4th', 1.25],
    ['mp3', 1.25],
    ['2026', 4 * 1.4 + 0.25],
    ['21:04', 4 * 1.4 + 0.25],
  ])('weighs %s as %f', (word, weight) => {
    expect(wordWeight(word)).toBeCloseTo(weight);
  });

  it.each(['first,', 'list;', 'note:', 'pause—', 'range–', 'said,”', 'aside,)'])(
    'adds a pause after %s',
    (word) => {
      expect(wordWeight(word)).toBeCloseTo(wordWeight(word.replace(/[,;:—–]/, '')) + 1.3);
    },
  );

  it('gives a word of no letters only its base and pause weight', () => {
    expect(wordWeight('…')).toBeCloseTo(0.25);
    expect(wordWeight('—')).toBeCloseTo(0.25 + 1.3);
  });
});

describe('wordStarts', () => {
  it('spreads the speech over the words by weight', () => {
    expect(wordStarts(['Two', 'emails', 'need', 'you.'], 1200)).toEqual([0, 250, 700, 950]);
  });

  it('starts every word at once when there is no speech to spread', () => {
    expect(wordStarts(['…', '…'], 0)).toEqual([0, 0]);
  });

  it('has nothing to time in an empty sentence', () => {
    expect(wordStarts([], 900)).toEqual([]);
  });
});

describe('wordAt', () => {
  const starts = [0, 250, 700, 950];

  it.each([
    [0, 0],
    [249, 0],
    [250, 1],
    [699, 1],
    [700, 2],
    [950, 3],
    [5000, 3],
  ])('at %i ms lights word %i', (elapsed, word) => {
    expect(wordAt(starts, elapsed)).toBe(word);
  });

  it('lights nothing before the sentence starts or in an empty sentence', () => {
    expect(wordAt(starts, -1)).toBeNull();
    expect(wordAt([], 10)).toBeNull();
  });
});
