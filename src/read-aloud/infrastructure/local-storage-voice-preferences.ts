import { isString, nullable, objectOf, parseJson } from '../../shared/checks.ts';
import type { VoicePreferences } from '../application/ports.ts';
import type { VoiceChoice } from '../domain/voice.ts';

export const VOICE_KEY = 'read-aloud.voice.v1';

function isName(value: unknown): value is string {
  return isString(value) && value.length > 0;
}

const isStoredChoice = objectOf({ name: isName, speaker: nullable(isString) });

export class LocalStorageVoicePreferences implements VoicePreferences {
  readonly #storage: Storage;

  constructor(storage: Storage) {
    this.#storage = storage;
  }

  load(): VoiceChoice | null {
    const stored = parseJson(this.#storage.getItem(VOICE_KEY));
    return isStoredChoice(stored) ? stored : null;
  }

  save(choice: VoiceChoice | null): void {
    if (choice === null) {
      this.#storage.removeItem(VOICE_KEY);
    } else {
      this.#storage.setItem(VOICE_KEY, JSON.stringify(choice));
    }
  }
}
