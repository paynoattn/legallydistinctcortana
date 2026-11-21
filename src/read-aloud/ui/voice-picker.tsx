import { type ReactElement, useSyncExternalStore } from 'react';
import type { VoiceStore } from '../application/voice-store.ts';
import {
  languageLabel,
  type Voice,
  type VoiceChoice,
  voiceLabel,
  voiceOptions,
} from '../domain/voice.ts';
import { onEscape } from './escape-key.ts';

interface VoicePickerProps {
  readonly store: VoiceStore;
  readonly onDismiss: () => void;
  readonly onPicked: () => void;
}

interface Option {
  readonly value: string;
  readonly label: string;
}

interface ChoiceProps {
  readonly label: string;
  readonly value: string;
  readonly options: readonly Option[];
  readonly onChoose: (value: string) => void;
  readonly onDismiss: () => void;
}

interface VoiceSelectsProps extends VoicePickerProps {
  readonly voices: readonly Voice[];
  readonly language: string | null;
  readonly choice: VoiceChoice | null;
}

function Choice({ label, value, options, onChoose, onDismiss }: ChoiceProps): ReactElement {
  return (
    <label>
      {label}
      <select
        value={value}
        onChange={(event) => {
          onChoose(event.currentTarget.value);
        }}
        onKeyDown={onEscape(onDismiss)}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function languageOptions(codes: readonly string[]): readonly Option[] {
  return codes.map((code) => ({ value: code, label: languageLabel(code) }));
}

function nameOptions(voices: readonly Voice[]): readonly Option[] {
  return [
    { value: '', label: 'Server default' },
    ...voices.map((voice) => ({ value: voice.name, label: voiceLabel(voice) })),
  ];
}

function speakerOptions(speakers: readonly string[]): readonly Option[] {
  return [
    { value: '', label: 'Default speaker' },
    ...speakers.map((name) => ({ value: name, label: name })),
  ];
}

function andThen(choose: (value: string) => void, after: () => void): (value: string) => void {
  return (value) => {
    choose(value);
    after();
  };
}

function VoiceSelects({
  store,
  onDismiss,
  onPicked,
  voices,
  language,
  choice,
}: VoiceSelectsProps): ReactElement {
  const options = voiceOptions(voices, language, choice);
  return (
    <fieldset className="voice-picker">
      <legend>Voice</legend>
      <Choice
        label="Language"
        value={options.language}
        options={languageOptions(options.languages)}
        onChoose={andThen(store.chooseLanguage, onPicked)}
        onDismiss={onDismiss}
      />
      <Choice
        label="Voice"
        value={options.name}
        options={nameOptions(options.voices)}
        onChoose={andThen(store.chooseVoice, onPicked)}
        onDismiss={onDismiss}
      />
      {options.speakers.length > 0 ? (
        <Choice
          label="Speaker"
          value={options.speaker}
          options={speakerOptions(options.speakers)}
          onChoose={andThen(store.chooseSpeaker, onPicked)}
          onDismiss={onDismiss}
        />
      ) : null}
    </fieldset>
  );
}

export function VoicePicker({ store, onDismiss, onPicked }: VoicePickerProps): ReactElement {
  const { list, language, choice } = useSyncExternalStore(store.subscribe, store.getSnapshot);
  switch (list.kind) {
    case 'loading':
      return <p className="voice-picker">Loading voices…</p>;
    case 'failed':
      return (
        <p className="voice-picker" role="alert">
          Voices are unavailable: {list.message}{' '}
          <button type="button" onClick={store.load} onKeyDown={onEscape(onDismiss)}>
            Retry
          </button>
        </p>
      );
    case 'ready':
      return (
        <VoiceSelects
          store={store}
          onDismiss={onDismiss}
          onPicked={onPicked}
          voices={list.voices}
          language={language}
          choice={choice}
        />
      );
  }
}
