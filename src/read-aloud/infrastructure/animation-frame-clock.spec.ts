import { describe, expect, it } from 'vitest';
import { AnimationFrameClock, type FrameSource } from './animation-frame-clock.ts';

class ScriptedFrames implements FrameSource {
  readonly performance = { now: (): number => this.time };
  readonly frames: (() => void)[] = [];
  time = 0;

  requestAnimationFrame(callback: () => void): number {
    this.frames.push(callback);
    return this.frames.length;
  }
}

describe('AnimationFrameClock', () => {
  it('reads the time from the page clock', () => {
    const source = new ScriptedFrames();
    const clock = new AnimationFrameClock(source);
    source.time = 1234.5;
    expect(clock.now()).toBe(1234.5);
  });

  it('runs a callback on the next animation frame', () => {
    const source = new ScriptedFrames();
    const ran: string[] = [];
    new AnimationFrameClock(source).nextFrame(() => ran.push('frame'));
    expect(ran).toEqual([]);
    source.frames[0]?.();
    expect(ran).toEqual(['frame']);
  });
});
