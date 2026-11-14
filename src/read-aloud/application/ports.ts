import type { MoodId } from '../domain/mood.ts';
import type { ReadingStatus, Selection } from '../domain/read-aloud-state.ts';
import type { Voice, VoiceChoice } from '../domain/voice.ts';

export interface VoiceCatalog {
  listVoices(): Promise<readonly Voice[]>;
}

export interface ReadAloudGateway extends VoiceCatalog {
  onSelection(listener: (selection: Selection) => void): void;
  onStatus(listener: (status: ReadingStatus) => void): void;
  read(mood: MoodId, voice: VoiceChoice | null): Promise<void>;
  stop(): Promise<void>;
  dismiss(): Promise<void>;
}

export interface VoicePreferences {
  load(): VoiceChoice | null;
  save(choice: VoiceChoice | null): void;
}
