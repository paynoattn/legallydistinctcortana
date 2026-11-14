import { describe, expect, it } from 'vitest';
import { describeLength, IDLE, WAITING } from './read-aloud-state.ts';

describe('read-aloud state', () => {
  it('starts waiting and idle', () => {
    expect(WAITING).toEqual({ kind: 'waiting' });
    expect(IDLE).toEqual({ kind: 'idle' });
  });

  it.each([
    [0, '0 characters'],
    [1, '1 character'],
    [2, '2 characters'],
    [1234, '1234 characters'],
  ])('describes %i as %s', (count, text) => {
    expect(describeLength(count)).toBe(text);
  });
});
