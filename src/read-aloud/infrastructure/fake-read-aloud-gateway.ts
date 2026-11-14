import type { ReadAloudGateway } from '../application/ports.ts';
import type { MoodId } from '../domain/mood.ts';
import { type ReadingStatus, type Selection, WAITING } from '../domain/read-aloud-state.ts';

export const NOTHING_SELECTED = 'Select some text first, then press the read-aloud shortcut.';

type Listener<T> = (value: T) => void;

function selectionFor(text: string | null): Selection {
  const collapsed = (text ?? '').trim().split(/\s+/).join(' ');
  return collapsed === ''
    ? { kind: 'failed', message: NOTHING_SELECTED }
    : { kind: 'ready', preview: collapsed, charCount: collapsed.length };
}

export class FakeReadAloudGateway implements ReadAloudGateway {
  readonly #selection: Selection;
  #onSelection: Listener<Selection> | undefined;
  #onStatus: Listener<ReadingStatus> | undefined;

  constructor(selectedText: string | null) {
    this.#selection = selectionFor(selectedText);
  }

  onSelection(listener: Listener<Selection>): void {
    this.#onSelection = listener;
    listener(this.#selection);
  }

  onStatus(listener: Listener<ReadingStatus>): void {
    this.#onStatus = listener;
  }

  read(mood: MoodId): Promise<void> {
    this.#onStatus?.({ kind: 'reading', mood });
    return Promise.resolve();
  }

  stop(): Promise<void> {
    this.#onStatus?.({ kind: 'stopped' });
    return Promise.resolve();
  }

  dismiss(): Promise<void> {
    this.#onSelection?.(WAITING);
    return Promise.resolve();
  }
}
