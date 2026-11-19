import { clampLevel } from '../domain/mark-pose.ts';
import { failed, HIDDEN, type OverlayState } from '../domain/overlay-state.ts';
import type { MotionPreference, OverlayGateway } from './ports.ts';

type Listener = () => void;

export interface OverlaySnapshot {
  readonly state: OverlayState;
  readonly level: number;
  readonly reduceMotion: boolean;
}

export class OverlayStore {
  readonly #listeners = new Set<Listener>();
  readonly #gateway: OverlayGateway;
  readonly #motion: MotionPreference;
  #askedToReduce = false;
  #snapshot: OverlaySnapshot;

  constructor(gateway: OverlayGateway, motion: MotionPreference) {
    this.#gateway = gateway;
    this.#motion = motion;
    this.#snapshot = { state: HIDDEN, level: 0, reduceMotion: motion.prefersReduced() };
    gateway.listen({
      state: ({ state, reduceMotion }) => {
        this.#askedToReduce = reduceMotion;
        this.#commit({ state, level: 0, reduceMotion: this.#reduced() });
      },
      level: (level) => {
        this.#commit({ ...this.#snapshot, level: clampLevel(level) });
      },
    });
    motion.onChange(() => {
      this.#commit({ ...this.#snapshot, reduceMotion: this.#reduced() });
    });
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  };

  readonly getSnapshot = (): OverlaySnapshot => this.#snapshot;

  readonly fix = (): void => {
    this.#gateway.fix().catch(this.#fail);
  };

  readonly dismiss = (): void => {
    this.#gateway.dismiss().catch(this.#fail);
  };

  readonly #fail = (error: unknown): void => {
    const { state } = failed(error instanceof Error ? error.message : String(error));
    this.#commit({ ...this.#snapshot, state, level: 0 });
  };

  #reduced(): boolean {
    return this.#askedToReduce || this.#motion.prefersReduced();
  }

  #commit(next: OverlaySnapshot): void {
    this.#snapshot = next;
    for (const listener of this.#listeners) {
      listener();
    }
  }
}
