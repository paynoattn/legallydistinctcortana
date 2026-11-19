import type { MotionPreference } from '../application/ports.ts';

export const REDUCED_MOTION_QUERY = '(prefers-reduced-motion: reduce)';

export interface MediaQuery {
  readonly matches: boolean;
  addEventListener(type: 'change', listener: () => void): void;
}

export class MediaQueryMotion implements MotionPreference {
  readonly #query: MediaQuery;

  constructor(query: MediaQuery) {
    this.#query = query;
  }

  prefersReduced(): boolean {
    return this.#query.matches;
  }

  onChange(listener: () => void): void {
    this.#query.addEventListener('change', listener);
  }
}
