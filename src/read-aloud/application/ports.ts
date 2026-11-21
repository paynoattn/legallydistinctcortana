import type { MoodId } from '../domain/mood.ts';
import type { ReadingStatus, Selection, SentenceReport } from '../domain/read-aloud-state.ts';
import type { Voice, VoiceChoice } from '../domain/voice.ts';

export interface VoiceCatalog {
  listVoices(): Promise<readonly Voice[]>;
}

export interface ReadRequest {
  readonly mood: MoodId;
  readonly voice: VoiceChoice | null;
  readonly fromSentence: number;
  readonly speed: number;
}

export interface ReadAloudGateway extends VoiceCatalog {
  onSelection(listener: (selection: Selection) => void): void;
  onStatus(listener: (status: ReadingStatus) => void): void;
  onSentence(listener: (report: SentenceReport) => void): void;
  read(request: ReadRequest): Promise<void>;
  stop(): Promise<void>;
  dismiss(): Promise<void>;
}

export interface VoicePreferences {
  load(): VoiceChoice | null;
  save(choice: VoiceChoice | null): void;
}

export interface ReaderPreferences {
  loadMood(): string | null;
  saveMood(mood: MoodId): void;
  loadTextSize(): string | null;
  saveTextSize(size: number): void;
  loadSpeed(): string | null;
  saveSpeed(speed: number): void;
}

export interface Clock {
  now(): number;
  nextFrame(callback: () => void): void;
}
