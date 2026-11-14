import type { MoodId } from '../domain/mood.ts';
import { IDLE, type ReadingStatus, type Selection, WAITING } from '../domain/read-aloud-state.ts';
import type { ReadAloudGateway } from './ports.ts';

type Listener = () => void;

export interface ReadAloudSnapshot {
  readonly selection: Selection;
  readonly status: ReadingStatus;
  readonly round: number;
}

export class ReadAloudStore {
  readonly #listeners = new Set<Listener>();
  readonly #gateway: ReadAloudGateway;
  #snapshot: ReadAloudSnapshot = { selection: WAITING, status: IDLE, round: 0 };

  constructor(gateway: ReadAloudGateway) {
    this.#gateway = gateway;
    gateway.onSelection((selection) => {
      this.#commit({ selection, status: IDLE, round: this.#snapshot.round + 1 });
    });
    gateway.onStatus((status) => {
      this.#commit({ ...this.#snapshot, status });
    });
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  };

  readonly getSnapshot = (): ReadAloudSnapshot => this.#snapshot;

  readonly choose = (mood: MoodId): void => {
    this.#commit({ ...this.#snapshot, status: { kind: 'reading', mood } });
    this.#gateway.read(mood).catch(this.#fail);
  };

  readonly stop = (): void => {
    this.#gateway.stop().catch(this.#fail);
  };

  readonly dismiss = (): void => {
    this.#gateway.dismiss().catch(this.#fail);
  };

  readonly #fail = (error: unknown): void => {
    this.#commit({ ...this.#snapshot, status: { kind: 'failed', message: messageOf(error) } });
  };

  #commit(next: ReadAloudSnapshot): void {
    this.#snapshot = next;
    for (const listener of this.#listeners) {
      listener();
    }
  }
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
