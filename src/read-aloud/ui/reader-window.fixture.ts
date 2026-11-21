import type {
  Clock,
  ReadAloudGateway,
  ReadRequest,
  VoicePreferences,
} from '../application/ports.ts';
import { ReaderStore } from '../application/reader-store.ts';
import { VoiceStore } from '../application/voice-store.ts';
import type { MoodId } from '../domain/mood.ts';
import type {
  ReadingStatus,
  Selection,
  Sentence,
  SentenceReport,
} from '../domain/read-aloud-state.ts';
import type { Voice, VoiceChoice } from '../domain/voice.ts';

type Act = (update: () => void) => unknown;

interface Read {
  readonly mood: MoodId;
  readonly voice: string | null;
  readonly from: number;
}

function paragraph(...texts: readonly string[]): readonly Sentence[] {
  return texts.map((text, index) => ({ text, startsParagraph: index === 0 }));
}

const PARAGRAPHS: readonly Sentence[] = [
  ...paragraph('Good morning.'),
  ...paragraph('Two emails need you.', 'Lunch is at noon.'),
];

const SPEECH_MS = 1200;

class ScriptedGateway implements ReadAloudGateway {
  readonly reads: Read[] = [];
  readonly calls: string[] = [];
  emitSelection: (selection: Selection) => void = () => undefined;
  emitStatus: (status: ReadingStatus) => void = () => undefined;
  emitSentence: (report: SentenceReport) => void = () => undefined;
  readonly #voices: readonly Voice[];
  readonly #act: Act;

  constructor(voices: readonly Voice[], act: Act) {
    this.#voices = voices;
    this.#act = act;
  }

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
    return Promise.resolve(this.#voices);
  }

  read({ mood, voice, fromSentence }: ReadRequest): Promise<void> {
    this.reads.push({ mood, voice: voice?.name ?? null, from: fromSentence });
    this.calls.push('read');
    this.emitStatus({ kind: 'reading' });
    return Promise.resolve();
  }

  stop(): Promise<void> {
    this.calls.push('stop');
    return Promise.resolve();
  }

  dismiss(): Promise<void> {
    this.calls.push('dismiss');
    return Promise.resolve();
  }

  select(sentences: readonly Sentence[] = PARAGRAPHS): void {
    this.#act(() => {
      this.emitSelection({ kind: 'ready', sentences });
    });
  }

  startSentence(index: number, speechMs = SPEECH_MS): void {
    this.#act(() => {
      this.emitSentence({ index, startsInMs: 0, speechMs });
    });
  }

  report(status: ReadingStatus): void {
    this.#act(() => {
      this.emitStatus(status);
    });
  }
}

class StillClock implements Clock {
  readonly now = (): number => 0;
  readonly nextFrame = (): void => undefined;
}

class NoVoicePreferences implements VoicePreferences {
  load(): VoiceChoice | null {
    return null;
  }

  readonly save = (): void => undefined;
}

export interface Stores {
  readonly gateway: ScriptedGateway;
  readonly clock: StillClock;
  readonly store: ReaderStore;
  readonly voices: VoiceStore;
}

export function stores(available: readonly Voice[], act: Act): Stores {
  const gateway = new ScriptedGateway(available, act);
  const clock = new StillClock();
  const voices = new VoiceStore({
    catalog: gateway,
    preferences: new NoVoicePreferences(),
    preferredLanguage: 'en-US',
  });
  voices.load();
  const store = new ReaderStore({
    gateway,
    clock,
    preferences: {
      loadMood: () => null,
      saveMood: () => undefined,
      loadTextSize: () => null,
      saveTextSize: () => undefined,
      loadSpeed: () => null,
      saveSpeed: () => undefined,
    },
    currentVoice: voices.current,
  });
  return { gateway, clock, store, voices };
}

export function focused(element: HTMLElement): boolean {
  return element.matches(':focus');
}
