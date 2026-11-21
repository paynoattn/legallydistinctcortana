import type { Clock } from '../application/ports.ts';

export interface FrameSource {
  readonly performance: { now(): number };
  requestAnimationFrame(callback: () => void): number;
}

export class AnimationFrameClock implements Clock {
  readonly #source: FrameSource;

  constructor(source: FrameSource) {
    this.#source = source;
  }

  now(): number {
    return this.#source.performance.now();
  }

  nextFrame(callback: () => void): void {
    this.#source.requestAnimationFrame(callback);
  }
}
