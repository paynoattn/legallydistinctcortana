import type { ReaderPreferences } from '../application/ports.ts';
import type { MoodId } from '../domain/mood.ts';

export const MOOD_KEY = 'read-aloud.mood.v1';
export const TEXT_SIZE_KEY = 'read-aloud.text-size.v1';
export const SPEED_KEY = 'read-aloud.speed.v1';

export class LocalStorageReaderPreferences implements ReaderPreferences {
  readonly #storage: Storage;

  constructor(storage: Storage) {
    this.#storage = storage;
  }

  loadMood(): string | null {
    return this.#storage.getItem(MOOD_KEY);
  }

  saveMood(mood: MoodId): void {
    this.#storage.setItem(MOOD_KEY, mood);
  }

  loadTextSize(): string | null {
    return this.#storage.getItem(TEXT_SIZE_KEY);
  }

  saveTextSize(size: number): void {
    this.#storage.setItem(TEXT_SIZE_KEY, String(size));
  }

  loadSpeed(): string | null {
    return this.#storage.getItem(SPEED_KEY);
  }

  saveSpeed(speed: number): void {
    this.#storage.setItem(SPEED_KEY, String(speed));
  }
}
