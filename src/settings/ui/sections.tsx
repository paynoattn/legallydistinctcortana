import type { ReactElement, ReactNode } from 'react';
import { type Connection, NOT_YET_CHECKED } from '../domain/connection.ts';
import type { Draft, FieldErrors, ShortcutField } from '../domain/settings.ts';
import { type Edit, TextField, Toggle } from './fields.tsx';
import { type Recorder, ShortcutRow } from './shortcut-row.tsx';

interface SectionProps {
  readonly title: string;
  readonly children: ReactNode;
}

function Section({ title, children }: SectionProps): ReactElement {
  return (
    <fieldset>
      <legend>{title}</legend>
      <div className="card">{children}</div>
    </fieldset>
  );
}

interface FieldsProps {
  readonly draft: Draft;
  readonly errors: FieldErrors;
  readonly onEdit: Edit;
}

interface ShortcutsProps {
  readonly draft: Draft;
  readonly errors: FieldErrors;
  readonly recording: ShortcutField | null;
  readonly recorder: Recorder;
}

const SHORTCUT_FIELDS: readonly ShortcutField[] = ['dictateShortcut', 'readAloudShortcut'];

export function ShortcutsSection({
  draft,
  errors,
  recording,
  recorder,
}: ShortcutsProps): ReactElement {
  return (
    <Section title="Shortcuts">
      {SHORTCUT_FIELDS.map((field) => (
        <ShortcutRow
          key={field}
          field={field}
          chord={draft[field]}
          recording={recording === field}
          error={errors[field]}
          recorder={recorder}
        />
      ))}
    </Section>
  );
}

interface ConnectionProps {
  readonly connection: Connection | null;
  readonly checking: boolean;
  readonly onCheck: () => void;
}

function ConnectionStatus({ connection, checking, onCheck }: ConnectionProps): ReactElement {
  const shown = connection ?? NOT_YET_CHECKED;
  return (
    <div className="row connection">
      <p role="status" className="connection-status" data-kind={shown.kind}>
        <span className="dot" aria-hidden="true" />
        {shown.summary}
      </p>
      <button type="button" disabled={checking} onClick={onCheck}>
        {checking ? 'Testing…' : 'Test connection'}
      </button>
    </div>
  );
}

export function VoiceServerSection({
  draft,
  errors,
  onEdit,
  ...connection
}: FieldsProps & ConnectionProps): ReactElement {
  const fields = { draft, onEdit };
  return (
    <Section title="Voice server">
      <TextField name="voiceHost" label="Host" error={errors.voiceHost} {...fields} />
      <TextField
        name="sttPort"
        label="Speech recognition port"
        error={errors.sttPort}
        {...fields}
      />
      <TextField name="ttsPort" label="Voice port" error={errors.ttsPort} {...fields} />
      <TextField
        name="sttLanguage"
        label="Dictation language"
        error={errors.sttLanguage}
        {...fields}
      />
      <ConnectionStatus {...connection} />
    </Section>
  );
}

export function IndicatorSection({ draft, errors, onEdit }: FieldsProps): ReactElement {
  return (
    <Section title="Indicator">
      <Toggle
        name="showOverlay"
        label="Show the listening indicator"
        draft={draft}
        error={errors.showOverlay}
        onEdit={onEdit}
      />
      <Toggle
        name="reduceMotion"
        label="Reduce motion"
        draft={draft}
        error={errors.reduceMotion}
        onEdit={onEdit}
      />
    </Section>
  );
}

export function StartupSection({ draft, errors, onEdit }: FieldsProps): ReactElement {
  return (
    <Section title="Startup">
      <Toggle
        name="startAtSignIn"
        label="Start at sign-in (hidden in the tray)"
        draft={draft}
        error={errors.startAtSignIn}
        onEdit={onEdit}
      />
    </Section>
  );
}
