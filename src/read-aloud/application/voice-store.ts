import {
  choiceFor,
  isOffered,
  languageOfChoice,
  languagesOf,
  pickLanguage,
  type Voice,
  type VoiceChoice,
  voicesIn,
} from '../domain/voice.ts';
import type { VoiceCatalog, VoicePreferences } from './ports.ts';

type Listener = () => void;

type VoiceList =
  | { readonly kind: 'loading' }
  | { readonly kind: 'ready'; readonly voices: readonly Voice[] }
  | { readonly kind: 'failed'; readonly message: string };

export interface VoiceSnapshot {
  readonly list: VoiceList;
  readonly language: string | null;
  readonly choice: VoiceChoice | null;
}

export interface VoiceStoreDeps {
  readonly catalog: VoiceCatalog;
  readonly preferences: VoicePreferences;
  readonly preferredLanguage: string;
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export class VoiceStore {
  readonly #listeners = new Set<Listener>();
  readonly #deps: VoiceStoreDeps;
  #snapshot: VoiceSnapshot;

  constructor(deps: VoiceStoreDeps) {
    this.#deps = deps;
    this.#snapshot = { list: { kind: 'loading' }, language: null, choice: deps.preferences.load() };
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  };

  readonly getSnapshot = (): VoiceSnapshot => this.#snapshot;

  readonly current = (): VoiceChoice | null => this.#snapshot.choice;

  readonly load = (): void => {
    this.#commit({ ...this.#snapshot, list: { kind: 'loading' } });
    this.#deps.catalog
      .listVoices()
      .then((voices) => {
        this.#loaded(voices);
      })
      .catch((error: unknown) => {
        this.#commit({ ...this.#snapshot, list: { kind: 'failed', message: messageOf(error) } });
      });
  };

  readonly chooseLanguage = (language: string): void => {
    this.#choose(language, choiceFor(voicesIn(this.#voices(), language)[0]));
  };

  readonly chooseVoice = (name: string): void => {
    const voice = this.#voices().find((candidate) => candidate.name === name);
    this.#choose(this.#snapshot.language, choiceFor(voice));
  };

  readonly chooseSpeaker = (speaker: string): void => {
    const { choice } = this.#snapshot;
    if (choice !== null) {
      this.#choose(this.#snapshot.language, {
        name: choice.name,
        speaker: speaker === '' ? null : speaker,
      });
    }
  };

  #loaded(voices: readonly Voice[]): void {
    const saved = this.#snapshot.choice;
    const choice = saved !== null && isOffered(voices, saved) ? saved : null;
    const language =
      languageOfChoice(voices, choice) ??
      pickLanguage(languagesOf(voices), this.#deps.preferredLanguage);
    this.#deps.preferences.save(choice);
    this.#commit({ list: { kind: 'ready', voices }, language, choice });
  }

  #voices(): readonly Voice[] {
    const { list } = this.#snapshot;
    return list.kind === 'ready' ? list.voices : [];
  }

  #choose(language: string | null, choice: VoiceChoice | null): void {
    this.#deps.preferences.save(choice);
    this.#commit({ ...this.#snapshot, language, choice });
  }

  #commit(next: VoiceSnapshot): void {
    this.#snapshot = next;
    for (const listener of this.#listeners) {
      listener();
    }
  }
}
