import type { MoodId } from '../domain/mood.ts';
import type {
  ReadingStatus,
  Selection,
  Sentence,
  SentenceReport,
} from '../domain/read-aloud-state.ts';
import type { Voice, VoiceChoice } from '../domain/voice.ts';
import type { Clock, ReadAloudGateway, ReaderPreferences, ReadRequest } from './ports.ts';
import { ReaderStore } from './reader-store.ts';

interface Read {
  readonly mood: MoodId;
  readonly voice: VoiceChoice | null;
  readonly from: number;
}

class ScriptedGateway implements ReadAloudGateway {
  readonly reads: Read[] = [];
  readonly speeds: number[] = [];
  readonly calls: string[] = [];
  failWith: unknown = undefined;
  announceReading = true;
  emitSelection: (selection: Selection) => void = () => undefined;
  emitStatus: (status: ReadingStatus) => void = () => undefined;
  emitSentence: (report: SentenceReport) => void = () => undefined;

  onSelection(listener: (selection: Selection) => void): void {
    this.emitSelection = listener;
  }

  onStatus(listener: (status: ReadingStatus) => void): void {
    this.emitStatus = listener;
  }

  onSentence(listener: (report: SentenceReport) => void): void {
    this.emitSentence = listener;
  }

  listVoices(): Promise<readonly Voice[]> {
    return Promise.resolve([]);
  }

  read({ mood, voice, fromSentence, speed }: ReadRequest): Promise<void> {
    this.reads.push({ mood, voice, from: fromSentence });
    this.speeds.push(speed);
    this.calls.push('read');
    if (this.announceReading) {
      this.emitStatus({ kind: 'reading' });
    }
    return this.outcome();
  }

  stop(): Promise<void> {
    this.calls.push('stop');
    return this.outcome();
  }

  dismiss(): Promise<void> {
    this.calls.push('dismiss');
    return this.outcome();
  }

  outcome(): Promise<void> {
    const { promise, reject, resolve } = Promise.withResolvers<undefined>();
    if (this.failWith === undefined) {
      resolve(undefined);
    } else {
      reject(this.failWith);
    }
    return promise;
  }
}

class FakeClock implements Clock {
  time = 0;
  frames: (() => void)[] = [];

  now(): number {
    return this.time;
  }

  nextFrame(callback: () => void): void {
    this.frames.push(callback);
  }

  advance(ms: number): void {
    this.time += ms;
    const due = this.frames;
    this.frames = [];
    for (const frame of due) {
      frame();
    }
  }
}

class MemoryPreferences implements ReaderPreferences {
  mood: MoodId | null = null;
  textSize: number | null = null;
  speed: number | null = null;

  loadMood(): string | null {
    return this.mood;
  }

  saveMood(mood: MoodId): void {
    this.mood = mood;
  }

  loadTextSize(): string | null {
    return this.textSize?.toString() ?? null;
  }

  saveTextSize(size: number): void {
    this.textSize = size;
  }

  loadSpeed(): string | null {
    return this.speed?.toString() ?? null;
  }

  saveSpeed(speed: number): void {
    this.speed = speed;
  }
}

export interface Rig {
  readonly gateway: ScriptedGateway;
  readonly clock: FakeClock;
  readonly preferences: MemoryPreferences;
  readonly store: ReaderStore;
  voice: VoiceChoice | null;
}

export interface Remembered {
  readonly mood?: MoodId;
  readonly textSize?: number;
  readonly speed?: number;
}

export function rig(remembered: Remembered = {}): Rig {
  const gateway = new ScriptedGateway();
  const clock = new FakeClock();
  const preferences = new MemoryPreferences();
  preferences.mood = remembered.mood ?? null;
  preferences.textSize = remembered.textSize ?? null;
  preferences.speed = remembered.speed ?? null;
  const state: { voice: VoiceChoice | null } = { voice: null };
  const store = new ReaderStore({
    gateway,
    clock,
    preferences,
    currentVoice: () => state.voice,
  });
  return {
    gateway,
    clock,
    preferences,
    store,
    get voice() {
      return state.voice;
    },
    set voice(choice) {
      state.voice = choice;
    },
  };
}

export function sentences(...texts: readonly string[]): readonly Sentence[] {
  return texts.map((text, index) => ({ text, startsParagraph: index === 0 }));
}

export const GOOD_MORNING = sentences('Good morning.', 'Two emails need you.');
export const THREE = sentences('One two.', 'Three four.', 'Five six.');

export function selected(text: readonly Sentence[] = THREE): Rig {
  const started = rig();
  started.gateway.emitSelection({ kind: 'ready', sentences: text });
  return started;
}

const SPEECH_MS = 2000;

export function at(started: Rig, index: number, speechMs = SPEECH_MS): void {
  started.gateway.emitSentence({ index, startsInMs: 0, speechMs });
}

export const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));
