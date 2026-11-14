import { z } from 'zod/mini';
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

const moodSchema = z.custom<MoodId>(isMoodId);

const selectionSchema = z.discriminatedUnion('kind', [
  z.object({
    kind: z.literal('ready'),
    preview: z.string(),
    charCount: z.int().check(z.nonnegative()),
  }),
  z.object({ kind: z.literal('failed'), message: z.string() }),
]);

const statusSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('reading'), mood: moodSchema }),
  z.object({ kind: z.literal('finished') }),
  z.object({ kind: z.literal('stopped') }),
  z.object({ kind: z.literal('failed'), message: z.string() }),
]);

const voicesSchema = z.array(
  z.object({
    name: z.string(),
    description: z.string(),
    languages: z.array(z.string()),
    speakers: z.array(z.string()),
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
    this.#subscribe<Selection>(SELECTION_EVENT, selectionSchema, (selection) => {
      live = true;
      listener(selection);
    })
      .then(() => this.#invoke(CURRENT_SELECTION_COMMAND))
      .then((payload) => {
        const parsed = selectionSchema.safeParse(payload);
        if (parsed.success && !live) {
          listener(parsed.data);
        }
      })
      .catch((error: unknown) => {
        listener(failure(messageOf(error)));
      });
  }

  onStatus(listener: (status: ReadingStatus) => void): void {
    this.#subscribe<ReadingStatus>(STATUS_EVENT, statusSchema, listener).catch((error: unknown) => {
      listener(failure(messageOf(error)));
    });
  }

  async read(mood: MoodId, voice: VoiceChoice | null): Promise<void> {
    await this.#invoke('read_aloud', {
      request: { mood, voice: voice?.name ?? null, speaker: voice?.speaker ?? null },
    });
  }

  async listVoices(): Promise<readonly Voice[]> {
    const parsed = voicesSchema.safeParse(await this.#invoke(LIST_VOICES_COMMAND));
    if (!parsed.success) {
      throw new Error(UNEXPECTED_MESSAGE);
    }
    return parsed.data;
  }

  async stop(): Promise<void> {
    await this.#invoke('stop_reading');
  }

  async dismiss(): Promise<void> {
    await this.#invoke('dismiss_read_aloud');
  }

  #subscribe<T>(
    event: string,
    schema: z.ZodMiniType<T>,
    listener: (value: T | Failure) => void,
  ): Promise<unknown> {
    return this.#listen(event, ({ payload }) => {
      const parsed = schema.safeParse(payload);
      listener(parsed.success ? parsed.data : failure(UNEXPECTED_MESSAGE));
    });
  }
}
