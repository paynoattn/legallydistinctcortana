import { describe, expect, it } from 'vitest';
import { paragraphsOf, WAITING, wordsOf } from './read-aloud-state.ts';

describe('read-aloud state', () => {
  it('starts waiting', () => {
    expect(WAITING).toEqual({ kind: 'waiting' });
  });

  it('groups sentences into paragraphs at each paragraph start', () => {
    expect(
      paragraphsOf([
        { text: 'Good morning.', startsParagraph: true },
        { text: 'Two emails need you.', startsParagraph: true },
        { text: 'Lunch is at noon.', startsParagraph: false },
      ]),
    ).toEqual([
      [{ index: 0, text: 'Good morning.' }],
      [
        { index: 1, text: 'Two emails need you.' },
        { index: 2, text: 'Lunch is at noon.' },
      ],
    ]);
  });

  it('opens a paragraph for a first sentence that does not claim to start one', () => {
    expect(
      paragraphsOf([
        { text: 'One.', startsParagraph: false },
        { text: 'Two.', startsParagraph: false },
      ]),
    ).toEqual([
      [
        { index: 0, text: 'One.' },
        { index: 1, text: 'Two.' },
      ],
    ]);
  });

  it('has no paragraphs without sentences', () => {
    expect(paragraphsOf([])).toEqual([]);
  });

  it('splits a sentence into its words', () => {
    expect(wordsOf('  Two  emails\tneed you. ')).toEqual(['Two', 'emails', 'need', 'you.']);
    expect(wordsOf('   ')).toEqual([]);
  });
});
