import {
  anyOf,
  arrayOf,
  integerIn,
  isString,
  objectOf,
  oneOf,
  type Check,
} from '../../shared/checks.ts';
import type { ReadAloudGateway } from '../application/ports.ts';
import { isMoodId, type MoodId } from '../domain/mood.ts';
import type { ReadingStatus, Selection } from '../domain/read-aloud-state.ts';
import type { Voice, VoiceChoice } from '../domain/voice.ts';

export type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;
export type Listen = (
  event: string,
  handler: (event: { readonly payload: unknown }) => void,
) => Promise<unknown>;

export const SELECTION_EVENT = 'read-aloud:selection';
export const STATUS_EVENT = 'read-aloud:status';
export const CURRENT_SELECTION_COMMAND = 'current_selection';
export const LIST_VOICES_COMMAND = 'list_voices';
export const UNEXPECTED_MESSAGE = 'The app sent an update this window does not understand.';

const isFailure = objectOf({ kind: oneOf('failed'), message: isString });

const isSelection: Check<Selection> = anyOf(
  objectOf({
    kind: oneOf('ready'),
    preview: isString,
    charCount: integerIn(0, Number.MAX_SAFE_INTEGER),
  }),
  isFailure,
);

const isStatus: Check<ReadingStatus> = anyOf(
  objectOf({ kind: oneOf('reading'), mood: isMoodId }),
  objectOf({ kind: oneOf('finished', 'stopped') }),
  isFailure,
);

const isVoices = arrayOf(
  objectOf({
    name: isString,
    description: isString,
    languages: arrayOf(isString),
    speakers: arrayOf(isString),
  }),
);

interface Failure {
  readonly kind: 'failed';
  readonly message: string;
}

function failure(message: string): Failure {
  return { kind: 'failed', message };
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export class TauriReadAloudGateway implements ReadAloudGateway {
  readonly #invoke: Invoke;
  readonly #listen: Listen;

  constructor(invoke: Invoke, listen: Listen) {
    this.#invoke = invoke;
    this.#listen = listen;
  }

  onSelection(listener: (selection: Selection) => void): void {
    let live = false;
    this.#subscribe<Selection>(SELECTION_EVENT, isSelection, (selection) => {
      live = true;
      listener(selection);
    })
      .then(() => this.#invoke(CURRENT_SELECTION_COMMAND))
      .then((payload) => {
        if (isSelection(payload) && !live) {
          listener(payload);
        }
      })
      .catch((error: unknown) => {
        listener(failure(messageOf(error)));
      });
  }

  onStatus(listener: (status: ReadingStatus) => void): void {
    this.#subscribe<ReadingStatus>(STATUS_EVENT, isStatus, listener).catch((error: unknown) => {
      listener(failure(messageOf(error)));
    });
  }

  async read(mood: MoodId, voice: VoiceChoice | null): Promise<void> {
    await this.#invoke('read_aloud', {
      request: { mood, voice: voice?.name ?? null, speaker: voice?.speaker ?? null },
    });
  }

  async listVoices(): Promise<readonly Voice[]> {
    const voices = await this.#invoke(LIST_VOICES_COMMAND);
    if (!isVoices(voices)) {
      throw new Error(UNEXPECTED_MESSAGE);
    }
    return voices;
  }

  async stop(): Promise<void> {
    await this.#invoke('stop_reading');
  }

  async dismiss(): Promise<void> {
    await this.#invoke('dismiss_read_aloud');
  }

  #subscribe<T>(
    event: string,
    check: Check<T>,
    listener: (value: T | Failure) => void,
  ): Promise<unknown> {
    return this.#listen(event, ({ payload }) => {
      listener(check(payload) ? payload : failure(UNEXPECTED_MESSAGE));
    });
  }
}
