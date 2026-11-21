import { type ReactElement, useSyncExternalStore } from 'react';
import type { Popover, ReaderStore } from '../application/reader-store.ts';
import type { VoiceStore } from '../application/voice-store.ts';
import { type MoodId, moodLabel } from '../domain/mood.ts';
import { type ReaderStatus, statusLine } from '../domain/playback.ts';
import { speedLabel } from '../domain/speed.ts';
import { chosenVoiceLabel } from '../domain/voice.ts';
import { focusOnMount } from './focus-on-mount.ts';
import { NEXT, PAUSE, PLAY, PREVIOUS } from './icon-paths.ts';
import { Icon } from './icons.tsx';
import { MoodMenu } from './mood-menu.tsx';
import { VoicePicker } from './voice-picker.tsx';

interface ControlBarProps {
  readonly store: ReaderStore;
  readonly voices: VoiceStore;
  readonly status: ReaderStatus;
  readonly sentence: number;
  readonly count: number;
  readonly mood: MoodId;
  readonly speed: number;
  readonly popover: Popover;
}

interface PlaybackButtonsProps {
  readonly store: ReaderStore;
  readonly reading: boolean;
}

interface ProgressProps {
  readonly done: number;
  readonly count: number;
  readonly finished: boolean;
}

interface MoodButtonProps {
  readonly store: ReaderStore;
  readonly mood: MoodId;
  readonly open: boolean;
}

interface VoiceButtonProps {
  readonly store: ReaderStore;
  readonly voices: VoiceStore;
  readonly open: boolean;
}

const PERCENT = 100;

function Progress({ done, count, finished }: ProgressProps): ReactElement {
  return (
    <div
      className={finished ? 'reader-progress is-done' : 'reader-progress'}
      role="progressbar"
      aria-label="Sentences read"
      aria-valuemin={0}
      aria-valuemax={count}
      aria-valuenow={done}
    >
      <i style={{ width: `${String(count === 0 ? 0 : (done / count) * PERCENT)}%` }} />
    </div>
  );
}

function MoodButton({ store, mood, open }: MoodButtonProps): ReactElement {
  return (
    <div className="reader-anchor">
      <button
        type="button"
        className="reader-button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={`Mood: ${moodLabel(mood)}`}
        onClick={() => {
          store.togglePopover('mood');
        }}
      >
        <span className="mood-swatch" data-mood={mood} />
        {moodLabel(mood)}
      </button>
      {open ? (
        <MoodMenu current={mood} onChoose={store.chooseMood} onClose={store.closePopover} />
      ) : null}
    </div>
  );
}

function VoiceButton({ store, voices, open }: VoiceButtonProps): ReactElement {
  const { list, choice } = useSyncExternalStore(voices.subscribe, voices.getSnapshot);
  const label = chosenVoiceLabel(list.kind === 'ready' ? list.voices : [], choice);
  return (
    <div className="reader-anchor">
      <button
        type="button"
        className="reader-button voice-button"
        aria-expanded={open}
        aria-label={`Voice: ${label}`}
        onClick={() => {
          store.togglePopover('voice');
        }}
      >
        {label}
      </button>
      {open ? (
        <div className="reader-popover">
          <VoicePicker
            store={voices}
            onDismiss={store.closePopover}
            onPicked={store.voiceChanged}
          />
        </div>
      ) : null}
    </div>
  );
}

function PlaybackButtons({ store, reading }: PlaybackButtonsProps): ReactElement {
  return (
    <>
      <button
        type="button"
        className="reader-button"
        aria-label="Previous sentence"
        onClick={store.previous}
      >
        <Icon d={PREVIOUS} />
      </button>
      <button
        type="button"
        className="reader-button reader-play"
        aria-label={reading ? 'Pause' : 'Play'}
        ref={focusOnMount}
        onClick={store.togglePlay}
      >
        <Icon d={reading ? PAUSE : PLAY} />
      </button>
      <button
        type="button"
        className="reader-button"
        aria-label="Next sentence"
        onClick={store.next}
      >
        <Icon d={NEXT} />
      </button>
    </>
  );
}

function KeyHint(): ReactElement {
  return (
    <p className="reader-hint">
      <kbd>Space</kbd> pauses. <kbd>←</kbd> <kbd>→</kbd> skip sentences. <kbd>1</kbd>–<kbd>9</kbd>{' '}
      pick a mood. Click a word to go there. <kbd>Esc</kbd> closes.
    </p>
  );
}

export function ControlBar({
  store,
  voices,
  status,
  sentence,
  count,
  mood,
  speed,
  popover,
}: ControlBarProps): ReactElement {
  const finished = status.kind === 'finished';
  return (
    <footer className="reader-bar">
      <Progress done={finished ? count : sentence} count={count} finished={finished} />
      <div className="reader-row">
        <PlaybackButtons store={store} reading={status.kind === 'reading'} />
        <span className="reader-separator" />
        <button
          type="button"
          className="reader-button"
          aria-label={`Speed: ${speedLabel(speed)}`}
          onClick={store.cycleSpeed}
        >
          {speedLabel(speed)}
        </button>
        <MoodButton store={store} mood={mood} open={popover === 'mood'} />
        <VoiceButton store={store} voices={voices} open={popover === 'voice'} />
        <p className="reader-status" role="status">
          {statusLine(status, sentence, count)}
        </p>
      </div>
      <KeyHint />
    </footer>
  );
}
