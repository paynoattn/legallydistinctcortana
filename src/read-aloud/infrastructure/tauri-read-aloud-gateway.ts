import {
  anyOf,
  arrayOf,
  integerIn,
  isBoolean,
  isString,
  objectOf,
  oneOf,
  type Check,
} from '../../shared/checks.ts';
import type { ReadAloudGateway, ReadRequest } from '../application/ports.ts';
import type { ReadingStatus, Selection, SentenceReport } from '../domain/read-aloud-state.ts';
import type { Voice } from '../domain/voice.ts';
import {
  CURRENT_SELECTION_COMMAND,
  DISMISS_COMMAND,
  LIST_VOICES_COMMAND,
  READ_COMMAND,
  SELECTION_EVENT,
  SENTENCE_EVENT,
  STATUS_EVENT,
  STOP_COMMAND,
} from './read-aloud-ipc-names.ts';

export type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;
export type Listen = (
  event: string,
  handler: (event: { readonly payload: unknown }) => void,
) => Promise<unknown>;

export const UNEXPECTED_MESSAGE = 'The app sent an update this window does not understand.';

const isFailure = objectOf({ kind: oneOf('failed'), message: isString });

const isWholeNumber = integerIn(0, Number.MAX_SAFE_INTEGER);

const isSelection: Check<Selection> = anyOf(
  objectOf({
    kind: oneOf('ready'),
    sentences: arrayOf(objectOf({ text: isString, startsParagraph: isBoolean })),
  }),
  isFailure,
);

const isStatus: Check<ReadingStatus> = anyOf(
  objectOf({ kind: oneOf('reading', 'finished', 'stopped') }),
  isFailure,
);

const isSentenceReport: Check<SentenceReport> = objectOf({
  index: isWholeNumber,
  startsInMs: isWholeNumber,
  speechMs: isWholeNumber,
});

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

  onSentence(listener: (report: SentenceReport) => void): void {
    this.#listen(SENTENCE_EVENT, ({ payload }) => {
      if (isSentenceReport(payload)) {
        listener(payload);
      }
    }).catch(() => undefined);
  }

  async read({ mood, voice, fromSentence, speed }: ReadRequest): Promise<void> {
    await this.#invoke(READ_COMMAND, {
      request: {
        mood,
        voice: voice?.name ?? null,
        speaker: voice?.speaker ?? null,
        fromSentence,
        speed,
      },
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
    await this.#invoke(STOP_COMMAND);
  }

  async dismiss(): Promise<void> {
    await this.#invoke(DISMISS_COMMAND);
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
