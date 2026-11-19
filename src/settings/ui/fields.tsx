import type { ReactElement } from 'react';
import type { Draft } from '../domain/settings.ts';
import { describedBy, errorIdFor } from './field-ids.ts';

export type Edit = <K extends keyof Draft>(field: K, value: Draft[K]) => void;
type TextName = 'voiceHost' | 'sttPort' | 'ttsPort' | 'sttLanguage';
type ToggleName = 'reduceMotion' | 'showOverlay' | 'startAtSignIn';

interface FieldErrorProps {
  readonly id: string;
  readonly message: string | undefined;
}

export function FieldError({ id, message }: FieldErrorProps): ReactElement {
  return (
    <p id={id} className="field-error">
      {message}
    </p>
  );
}

interface TextFieldProps {
  readonly name: TextName;
  readonly label: string;
  readonly draft: Draft;
  readonly error: string | undefined;
  readonly onEdit: Edit;
}

export function TextField({ name, label, draft, error, onEdit }: TextFieldProps): ReactElement {
  return (
    <div className="row">
      <label htmlFor={name}>{label}</label>
      <input
        id={name}
        type="text"
        value={draft[name]}
        spellCheck={false}
        autoComplete="off"
        aria-invalid={error !== undefined}
        aria-describedby={describedBy(name, error)}
        onChange={(event) => {
          onEdit(name, event.currentTarget.value);
        }}
      />
      <FieldError id={errorIdFor(name)} message={error} />
    </div>
  );
}

interface ToggleProps {
  readonly name: ToggleName;
  readonly label: string;
  readonly draft: Draft;
  readonly error: string | undefined;
  readonly onEdit: Edit;
}

export function Toggle({ name, label, draft, error, onEdit }: ToggleProps): ReactElement {
  return (
    <div className="row">
      <label htmlFor={name}>{label}</label>
      <input
        id={name}
        type="checkbox"
        role="switch"
        checked={draft[name]}
        aria-describedby={describedBy(name, error)}
        onChange={(event) => {
          onEdit(name, event.currentTarget.checked);
        }}
      />
      <FieldError id={errorIdFor(name)} message={error} />
    </div>
  );
}
