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
}

interface LanguageSelectProps extends VoicePickerProps {
  readonly languages: readonly string[];
  readonly language: string;
}

interface NameSelectProps extends VoicePickerProps {
  readonly voices: readonly Voice[];
  readonly name: string;
}

interface SpeakerSelectProps extends VoicePickerProps {
  readonly speakers: readonly string[];
  readonly speaker: string;
}

interface VoiceSelectsProps extends VoicePickerProps {
  readonly voices: readonly Voice[];
  readonly language: string | null;
  readonly choice: VoiceChoice | null;
}

function LanguageSelect({
  store,
  onDismiss,
  languages,
  language,
}: LanguageSelectProps): ReactElement {
  return (
    <label>
      Language
      <select
        value={language}
        onChange={(event) => {
          store.chooseLanguage(event.currentTarget.value);
        }}
        onKeyDown={onEscape(onDismiss)}
      >
        {languages.map((code) => (
          <option key={code} value={code}>
            {languageLabel(code)}
          </option>
        ))}
      </select>
    </label>
  );
}

function NameSelect({ store, onDismiss, voices, name }: NameSelectProps): ReactElement {
  return (
    <label>
      Voice
      <select
        value={name}
        onChange={(event) => {
          store.chooseVoice(event.currentTarget.value);
        }}
        onKeyDown={onEscape(onDismiss)}
      >
        <option value="">Server default</option>
        {voices.map((voice) => (
          <option key={voice.name} value={voice.name}>
            {voiceLabel(voice)}
          </option>
        ))}
      </select>
    </label>
  );
}

function SpeakerSelect({ store, onDismiss, speakers, speaker }: SpeakerSelectProps): ReactElement {
  return (
    <label>
      Speaker
      <select
        value={speaker}
        onChange={(event) => {
          store.chooseSpeaker(event.currentTarget.value);
        }}
        onKeyDown={onEscape(onDismiss)}
      >
        <option value="">Default speaker</option>
        {speakers.map((name) => (
          <option key={name} value={name}>
            {name}
          </option>
        ))}
      </select>
    </label>
  );
}

function VoiceSelects({
  store,
  onDismiss,
  voices,
  language,
  choice,
}: VoiceSelectsProps): ReactElement {
  const options = voiceOptions(voices, language, choice);
  return (
    <fieldset className="voice-picker">
      <legend>Voice</legend>
      <LanguageSelect
        store={store}
        onDismiss={onDismiss}
        languages={options.languages}
        language={options.language}
      />
      <NameSelect store={store} onDismiss={onDismiss} voices={options.voices} name={options.name} />
      {options.speakers.length > 0 ? (
        <SpeakerSelect
          store={store}
          onDismiss={onDismiss}
          speakers={options.speakers}
          speaker={options.speaker}
        />
      ) : null}
    </fieldset>
  );
}

export function VoicePicker({ store, onDismiss }: VoicePickerProps): ReactElement {
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
          voices={list.voices}
          language={language}
          choice={choice}
        />
      );
  }
}
