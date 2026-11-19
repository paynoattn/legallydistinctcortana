import type { KeyboardEvent, ReactElement } from 'react';
import { CANCEL_CODE, chordOf } from '../domain/chord.ts';
import { SHORTCUT_LABELS, type ShortcutField } from '../domain/settings.ts';
import { describedBy, errorIdFor } from './field-ids.ts';
import { FieldError } from './fields.tsx';

export interface Recorder {
  readonly startRecording: (field: ShortcutField) => void;
  readonly record: (chord: string) => void;
  readonly stopRecording: () => void;
  readonly resetShortcut: (field: ShortcutField) => void;
}

interface ShortcutRowProps {
  readonly field: ShortcutField;
  readonly chord: string;
  readonly recording: boolean;
  readonly error: string | undefined;
  readonly recorder: Recorder;
}

interface ChordProps {
  readonly field: ShortcutField;
  readonly chord: string;
}

function Chord({ field, chord }: ChordProps): ReactElement {
  return (
    <kbd className="chord" data-testid={`chord-${field}`}>
      {chord.split('+').map((key, index) => (
        <span key={key}>
          {index > 0 ? '+' : null}
          <kbd>{key}</kbd>
        </span>
      ))}
    </kbd>
  );
}

function keyHandler(recorder: Recorder): (event: KeyboardEvent<HTMLButtonElement>) => void {
  return (event) => {
    event.preventDefault();
    if (event.code === CANCEL_CODE) {
      recorder.stopRecording();
      return;
    }
    const chord = chordOf(event);
    if (chord !== null) {
      recorder.record(chord);
    }
  };
}

interface ChangeButtonProps {
  readonly field: ShortcutField;
  readonly recording: boolean;
  readonly error: string | undefined;
  readonly recorder: Recorder;
}

function ChangeButton({ field, recording, error, recorder }: ChangeButtonProps): ReactElement {
  return (
    <button
      type="button"
      className="change"
      data-recording={recording}
      aria-label={recording ? undefined : `Change ${SHORTCUT_LABELS[field]}`}
      aria-describedby={describedBy(field, error)}
      onClick={(event) => {
        event.currentTarget.focus();
        recorder.startRecording(field);
      }}
      onKeyDown={recording ? keyHandler(recorder) : undefined}
      onBlur={recorder.stopRecording}
    >
      {recording ? 'Press a shortcut, or Esc to cancel' : 'Change'}
    </button>
  );
}

export function ShortcutRow({
  field,
  chord,
  recording,
  error,
  recorder,
}: ShortcutRowProps): ReactElement {
  const label = SHORTCUT_LABELS[field];
  return (
    <div className="row shortcut">
      <div className="shortcut-what">
        <span>{label}</span>
        <Chord field={field} chord={chord} />
      </div>
      <ChangeButton field={field} recording={recording} error={error} recorder={recorder} />
      <button
        type="button"
        aria-label={`Reset ${label}`}
        onClick={() => {
          recorder.resetShortcut(field);
        }}
      >
        Reset
      </button>
      <FieldError id={errorIdFor(field)} message={error} />
    </div>
  );
}
