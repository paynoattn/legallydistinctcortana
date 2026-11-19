import { describe, expect, it, vi } from 'vitest';
import { type MediaQuery, MediaQueryMotion, REDUCED_MOTION_QUERY } from './media-query-motion.ts';

class FakeQuery implements MediaQuery {
  matches = false;
  readonly listeners: { type: string; listener: () => void }[] = [];

  addEventListener(type: 'change', listener: () => void): void {
    this.listeners.push({ type, listener });
  }
}

describe('MediaQueryMotion', () => {
  it('asks the reduced motion media query', () => {
    expect(REDUCED_MOTION_QUERY).toBe('(prefers-reduced-motion: reduce)');
  });

  it('reads whether the query matches', () => {
    const query = new FakeQuery();
    const motion = new MediaQueryMotion(query);
    expect(motion.prefersReduced()).toBe(false);
    query.matches = true;
    expect(motion.prefersReduced()).toBe(true);
  });

  it('listens for changes', () => {
    const query = new FakeQuery();
    const listener = vi.fn<() => void>();
    new MediaQueryMotion(query).onChange(listener);
    expect(query.listeners).toEqual([{ type: 'change', listener }]);
  });
});
