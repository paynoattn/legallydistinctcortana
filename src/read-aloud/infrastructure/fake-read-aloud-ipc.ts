import { isString, objectOf } from '../../shared/checks.ts';
import type { Sentence } from '../domain/read-aloud-state.ts';
import type { Voice } from '../domain/voice.ts';
import {
  CURRENT_SELECTION_COMMAND,
  LIST_VOICES_COMMAND,
  READ_COMMAND,
  SELECTION_EVENT,
  STATUS_EVENT,
  STOP_COMMAND,
} from './read-aloud-ipc-names.ts';

export const NOTHING_SELECTED = 'Select some text first, then press the read-aloud shortcut.';
const FAKE_EVENT_PREFIX = 'fake-ipc:';
export const INVOKED_EVENT = 'fake-ipc:invoked';
export const CAPTURE_EVENT = 'fake-ipc:capture';

export const FAKE_VOICES: readonly Voice[] = [
  {
    name: 'en_US-amy-medium',
    description: 'amy (medium)',
    languages: ['en_US'],
    speakers: [],
  },
  {
    name: 'en_US-libritts-high',
    description: 'libritts (high)',
    languages: ['en_US'],
    speakers: ['p239', 'p240'],
  },
  {
    name: 'de_DE-thorsten-medium',
    description: 'thorsten (medium)',
    languages: ['de_DE'],
    speakers: [],
  },
];

const isReadArgs = objectOf({ request: objectOf({ mood: isString }) });

export function sentencesOf(text: string): readonly Sentence[] {
  return text.split('\n').flatMap((line) =>
    line
      .split(/(?<=[.!?…])\s/)
      .map((sentence) => sentence.trim())
      .filter((sentence) => sentence !== '')
      .map((sentence, index) => ({ text: sentence, startsParagraph: index === 0 })),
  );
}

export function selectionFor(text: string | null): unknown {
  const sentences = sentencesOf(text ?? '');
  return sentences.length === 0
    ? { kind: 'failed', message: NOTHING_SELECTED }
    : { kind: 'ready', sentences };
}

export class FakeReadAloudIpc {
  readonly #target: EventTarget;
  readonly #selectedText: string | null;

  constructor(target: EventTarget, selectedText: string | null) {
    this.#target = target;
    this.#selectedText = selectedText;
    target.addEventListener(CAPTURE_EVENT, (event) => {
      const text: unknown = event instanceof CustomEvent ? event.detail : null;
      this.#emit(SELECTION_EVENT, selectionFor(isString(text) ? text : null));
    });
  }

  readonly invoke = (command: string, args?: Record<string, unknown>): Promise<unknown> => {
    this.#target.dispatchEvent(new CustomEvent(INVOKED_EVENT, { detail: { command, args } }));
    return Promise.resolve(this.#answer(command, args));
  };

  readonly listen = (
    event: string,
    handler: (event: { readonly payload: unknown }) => void,
  ): Promise<unknown> => {
    this.#target.addEventListener(`${FAKE_EVENT_PREFIX}${event}`, (fired) => {
      handler({ payload: fired instanceof CustomEvent ? fired.detail : undefined });
    });
    return Promise.resolve(null);
  };

  #answer(command: string, args: Record<string, unknown> | undefined): unknown {
    switch (command) {
      case CURRENT_SELECTION_COMMAND:
        return selectionFor(this.#selectedText);
      case LIST_VOICES_COMMAND:
        return FAKE_VOICES;
      case READ_COMMAND:
        this.#emit(STATUS_EVENT, {
          kind: 'reading',
          mood: isReadArgs(args) ? args.request.mood : 'neutral',
        });
        return null;
      case STOP_COMMAND:
        this.#emit(STATUS_EVENT, { kind: 'stopped' });
        return null;
      default:
        return null;
    }
  }

  #emit(event: string, payload: unknown): void {
    this.#target.dispatchEvent(
      new CustomEvent(`${FAKE_EVENT_PREFIX}${event}`, { detail: payload }),
    );
  }
}
