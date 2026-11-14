import { z } from 'zod/mini';
import type { VoicePreferences } from '../application/ports.ts';
import type { VoiceChoice } from '../domain/voice.ts';

export const VOICE_KEY = 'read-aloud.voice.v1';

function parseJson(raw: string): unknown {
  try {
    const value: unknown = JSON.parse(raw);
    return value;
  } catch {
    return undefined;
  }
}

const storedChoice = z.catch(
  z.nullable(
    z.pipe(
      z.pipe(
        z.string(),
        z.transform((raw) => parseJson(raw)),
      ),
      z.object({
        name: z.string().check(z.minLength(1)),
        speaker: z.nullable(z.string()),
      }),
    ),
  ),
  null,
);

export class LocalStorageVoicePreferences implements VoicePreferences {
  readonly #storage: Storage;

  constructor(storage: Storage) {
    this.#storage = storage;
  }

  load(): VoiceChoice | null {
    return storedChoice.parse(this.#storage.getItem(VOICE_KEY));
  }

  save(choice: VoiceChoice | null): void {
    if (choice === null) {
      this.#storage.removeItem(VOICE_KEY);
    } else {
      this.#storage.setItem(VOICE_KEY, JSON.stringify(choice));
    }
  }
}
