import { type MoodId, storedMood } from '../domain/mood.ts';
import {
  positionAt,
  previousSentence,
  type ReaderStatus,
  resizedText,
  type SentenceTiming,
  storedTextSize,
} from '../domain/playback.ts';
import {
  type ReadingStatus,
  type Selection,
  type SentenceReport,
  WAITING,
  wordsOf,
} from '../domain/read-aloud-state.ts';
import type { VoiceChoice } from '../domain/voice.ts';
import { nextSpeed, storedSpeed } from '../domain/speed.ts';
import { wordStarts } from '../domain/word-timing.ts';
import type { Clock, ReadAloudGateway, ReaderPreferences } from './ports.ts';

type Listener = () => void;

export type Popover = 'mood' | 'voice' | null;

export interface ReaderSnapshot {
  readonly selection: Selection;
  readonly round: number;
  readonly mood: MoodId;
  readonly textSize: number;
  readonly speed: number;
  readonly status: ReaderStatus;
  readonly sentence: number;
  readonly word: number | null;
  readonly following: boolean;
  readonly popover: Popover;
}

export interface ReaderStoreDeps {
  readonly gateway: ReadAloudGateway;
  readonly clock: Clock;
  readonly preferences: ReaderPreferences;
  readonly currentVoice: () => VoiceChoice | null;
}

const IDLE: ReaderStatus = { kind: 'idle' };
const READING: ReaderStatus = { kind: 'reading' };
const PAUSED: ReaderStatus = { kind: 'paused' };
const FINISHED: ReaderStatus = { kind: 'finished' };

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export class ReaderStore {
  readonly #listeners = new Set<Listener>();
  readonly #deps: ReaderStoreDeps;
  readonly #timings = new Map<number, SentenceTiming>();
  #words: readonly (readonly string[])[] = [];
  #from = 0;
  #awaitingStart = false;
  #frameRequested = false;
  #snapshot: ReaderSnapshot;

  constructor(deps: ReaderStoreDeps) {
    this.#deps = deps;
    this.#snapshot = {
      selection: WAITING,
      round: 0,
      mood: storedMood(deps.preferences.loadMood()),
      textSize: storedTextSize(deps.preferences.loadTextSize()),
      speed: storedSpeed(deps.preferences.loadSpeed()),
      status: IDLE,
      sentence: 0,
      word: null,
      following: true,
      popover: null,
    };
    deps.gateway.onSelection(this.#select);
    deps.gateway.onStatus(this.#report);
    deps.gateway.onSentence(this.#time);
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  };

  readonly getSnapshot = (): ReaderSnapshot => this.#snapshot;

  readonly togglePlay = (): void => {
    switch (this.#snapshot.status.kind) {
      case 'reading':
        this.#pause();
        return;
      case 'finished':
        this.#readFrom(0);
        return;
      default:
        this.#readFrom(this.#snapshot.sentence);
    }
  };

  readonly next = (): void => {
    const next = this.#snapshot.sentence + 1;
    if (next < this.#words.length) {
      this.#readFrom(next);
    }
  };

  readonly previous = (): void => {
    const { sentence, status } = this.#snapshot;
    const played = status.kind === 'reading' ? this.#played(sentence) : 0;
    this.#readFrom(previousSentence(Math.min(sentence, this.#words.length), played));
  };

  readonly readSentence = (index: number): void => {
    this.#readFrom(index);
  };

  readonly chooseMood = (mood: MoodId): void => {
    const changed = mood !== this.#snapshot.mood;
    this.#deps.preferences.saveMood(mood);
    this.#commit({ mood, popover: null });
    if (changed) {
      this.#restart();
    }
  };

  readonly cycleSpeed = (): void => {
    const speed = nextSpeed(this.#snapshot.speed);
    this.#deps.preferences.saveSpeed(speed);
    this.#commit({ speed, popover: null });
    this.#restart();
  };

  readonly voiceChanged = (): void => {
    this.#restart();
  };

  readonly larger = (): void => {
    this.#resize(1);
  };

  readonly smaller = (): void => {
    this.#resize(-1);
  };

  readonly stopFollowing = (): void => {
    if (this.#snapshot.following) {
      this.#commit({ following: false });
    }
  };

  readonly follow = (): void => {
    this.#commit({ following: true });
  };

  readonly togglePopover = (popover: Exclude<Popover, null>): void => {
    this.#commit({ popover: this.#snapshot.popover === popover ? null : popover });
  };

  readonly closePopover = (): void => {
    this.#commit({ popover: null });
  };

  readonly escape = (): void => {
    if (this.#snapshot.popover === null) {
      this.close();
    } else {
      this.closePopover();
    }
  };

  readonly close = (): void => {
    this.#awaitingStart = false;
    this.#commit({ status: this.#isReading() ? PAUSED : this.#snapshot.status, popover: null });
    const { gateway } = this.#deps;
    gateway
      .stop()
      .then(() => gateway.dismiss())
      .catch(this.#fail);
  };

  readonly #select = (selection: Selection): void => {
    this.#awaitingStart = false;
    this.#words =
      selection.kind === 'ready'
        ? selection.sentences.map((sentence) => wordsOf(sentence.text))
        : [];
    this.#commit({
      selection,
      round: this.#snapshot.round + 1,
      status: IDLE,
      sentence: 0,
      word: null,
      following: true,
      popover: null,
    });
    this.#readFrom(0);
  };

  readonly #report = (status: ReadingStatus): void => {
    if (status.kind === 'reading') {
      this.#awaitingStart = false;
      return;
    }
    if (this.#awaitingStart) {
      return;
    }
    if (status.kind === 'failed') {
      this.#commit({ status: { kind: 'failed', message: status.message }, word: null });
    } else if (this.#isReading()) {
      this.#ended(status.kind);
    }
  };

  readonly #time = (report: SentenceReport): void => {
    if (this.#awaitingStart || !this.#isReading()) {
      return;
    }
    this.#timings.set(report.index, {
      startAt: this.#deps.clock.now() + report.startsInMs,
      starts: wordStarts(this.#words[report.index] ?? [], report.speechMs),
    });
    this.#locate();
  };

  readonly #tick = (): void => {
    this.#frameRequested = false;
    if (this.#isReading()) {
      this.#locate();
      this.#requestFrame();
    }
  };

  readonly #fail = (error: unknown): void => {
    this.#commit({ status: { kind: 'failed', message: messageOf(error) }, word: null });
  };

  #ended(kind: 'finished' | 'stopped'): void {
    if (kind === 'finished') {
      this.#commit({ status: FINISHED, sentence: this.#words.length, word: null });
    } else {
      this.#commit({ status: PAUSED });
    }
  }

  #readFrom(index: number): void {
    if (this.#words.length === 0) {
      return;
    }
    const from = Math.min(Math.max(0, index), this.#words.length - 1);
    this.#timings.clear();
    this.#from = from;
    this.#awaitingStart = true;
    this.#commit({ status: READING, sentence: from, word: null, following: true });
    const { mood, speed } = this.#snapshot;
    this.#deps.gateway
      .read({ mood, voice: this.#deps.currentVoice(), fromSentence: from, speed })
      .catch(this.#fail);
    this.#requestFrame();
  }

  #restart(): void {
    const { status, sentence } = this.#snapshot;
    this.#readFrom(status.kind === 'finished' ? 0 : sentence);
  }

  #pause(): void {
    this.#commit({ status: PAUSED });
    this.#deps.gateway.stop().catch(this.#fail);
  }

  #resize(direction: 1 | -1): void {
    const textSize = resizedText(this.#snapshot.textSize, direction);
    this.#deps.preferences.saveTextSize(textSize);
    this.#commit({ textSize });
  }

  #played(sentence: number): number {
    const timing = this.#timings.get(sentence);
    return timing === undefined ? 0 : Math.max(0, this.#deps.clock.now() - timing.startAt);
  }

  #isReading(): boolean {
    return this.#snapshot.status.kind === 'reading';
  }

  #locate(): void {
    const { sentence, word } = positionAt(this.#timings, this.#from, this.#deps.clock.now());
    if (sentence !== this.#snapshot.sentence || word !== this.#snapshot.word) {
      this.#commit({ sentence, word });
    }
  }

  #requestFrame(): void {
    if (!this.#frameRequested) {
      this.#frameRequested = true;
      this.#deps.clock.nextFrame(this.#tick);
    }
  }

  #commit(change: Partial<ReaderSnapshot>): void {
    this.#snapshot = { ...this.#snapshot, ...change };
    for (const listener of this.#listeners) {
      listener();
    }
  }
}
