import { describe, expect, it } from 'vitest';
import { nextFocusIndex } from './menu-navigation.ts';

describe('nextFocusIndex', () => {
  it.each([
    { key: 'ArrowDown', current: 0, count: 3, expected: 1 },
    { key: 'ArrowDown', current: 2, count: 3, expected: 0 },
    { key: 'ArrowUp', current: 2, count: 3, expected: 1 },
    { key: 'ArrowUp', current: 0, count: 3, expected: 2 },
    { key: 'Home', current: 2, count: 3, expected: 0 },
    { key: 'End', current: 0, count: 3, expected: 2 },
    { key: 'ArrowDown', current: -1, count: 3, expected: 1 },
    { key: 'ArrowUp', current: -1, count: 3, expected: 2 },
  ])('$key from $current of $count goes to $expected', ({ key, current, count, expected }) => {
    expect(nextFocusIndex(key, current, count)).toBe(expected);
  });

  it('ignores other keys', () => {
    expect(nextFocusIndex('a', 0, 3)).toBeUndefined();
    expect(nextFocusIndex('Enter', 1, 3)).toBeUndefined();
  });

  it.each(['ArrowDown', 'ArrowUp', 'Home', 'End'])('does nothing on %s with no items', (key) => {
    expect(nextFocusIndex(key, 0, 0)).toBeUndefined();
  });
});
