import { describe, expect, it } from 'vitest';
import { NORMAL_SPEED, nextSpeed, speedLabel, storedSpeed } from './speed.ts';

describe('reading speed', () => {
  it('cycles from normal through faster speeds to slower and back', () => {
    const seen = [NORMAL_SPEED];
    for (let step = 0; step < 5; step += 1) {
      seen.push(nextSpeed(seen.at(-1) ?? NORMAL_SPEED));
    }
    expect(seen).toEqual([1000, 1250, 1500, 2000, 750, 1000]);
  });

  it('starts the cycle again from a speed it does not know', () => {
    expect(nextSpeed(1100)).toBe(1000);
  });

  it.each([
    [1000, '1×'],
    [1250, '1.25×'],
    [1500, '1.5×'],
    [2000, '2×'],
    [750, '0.75×'],
  ])('labels %i as %s', (speed, label) => {
    expect(speedLabel(speed)).toBe(label);
  });

  it.each([
    ['1500', 1500],
    ['750', 750],
  ])('keeps the stored speed %j', (stored, speed) => {
    expect(storedSpeed(stored)).toBe(speed);
  });

  it.each([null, '', '1100', 'fast', '0'])('falls back to normal for %j', (stored) => {
    expect(storedSpeed(stored)).toBe(NORMAL_SPEED);
  });
});
