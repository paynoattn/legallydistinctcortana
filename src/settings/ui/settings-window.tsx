import { type ReactElement, useSyncExternalStore } from 'react';
import type { Notice, SettingsSnapshot, SettingsStore } from '../application/settings-store.ts';
import { type Draft, errorsFor, sameDraft, validate } from '../domain/settings.ts';
import {
  IndicatorSection,
  ShortcutsSection,
  StartupSection,
  VoiceServerSection,
} from './sections.tsx';
import './settings.css';

interface SettingsWindowProps {
  readonly store: SettingsStore;
}

interface SettingsFormProps {
  readonly store: SettingsStore;
  readonly snapshot: SettingsSnapshot;
  readonly saved: Draft;
  readonly draft: Draft;
}

function noticeText(notice: Notice): string {
  switch (notice.kind) {
    case 'saved':
      return 'Saved.';
    case 'failed':
      return notice.message;
    case 'none':
      return '';
  }
}

function canSave(snapshot: SettingsSnapshot, saved: Draft, draft: Draft): boolean {
  return !snapshot.saving && !sameDraft(saved, draft) && Object.keys(validate(draft)).length === 0;
}

function SettingsForm({ store, snapshot, saved, draft }: SettingsFormProps): ReactElement {
  const errors = errorsFor(draft, snapshot.coreErrors);
  const fields = { draft, errors, onEdit: store.edit };
  return (
    <form
      className="settings-form"
      noValidate
      onSubmit={(event) => {
        event.preventDefault();
        store.save();
      }}
    >
      <ShortcutsSection
        draft={draft}
        errors={errors}
        recording={snapshot.recording}
        recorder={store}
      />
      <VoiceServerSection
        {...fields}
        connection={snapshot.connection}
        checking={snapshot.checking}
        onCheck={store.checkConnection}
      />
      <IndicatorSection {...fields} />
      <StartupSection {...fields} />
      <div className="actions">
        <p role="status" className="notice" data-kind={snapshot.notice.kind}>
          {noticeText(snapshot.notice)}
        </p>
        <button type="button" disabled={sameDraft(saved, draft)} onClick={store.revert}>
          Revert
        </button>
        <button type="submit" className="primary" disabled={!canSave(snapshot, saved, draft)}>
          Save
        </button>
      </div>
    </form>
  );
}

function SettingsBody({ store }: SettingsWindowProps): ReactElement {
  const snapshot = useSyncExternalStore(store.subscribe, store.getSnapshot);
  const { form } = snapshot;
  switch (form.kind) {
    case 'loading':
      return <p>Loading settings…</p>;
    case 'failed':
      return <p role="alert">Settings could not be loaded. {form.message}</p>;
    case 'ready':
      return (
        <SettingsForm store={store} snapshot={snapshot} saved={form.saved} draft={form.draft} />
      );
  }
}

export function SettingsWindow({ store }: SettingsWindowProps): ReactElement {
  return (
    <main className="settings" aria-labelledby="settings-title">
      <h1 id="settings-title">Settings</h1>
      <SettingsBody store={store} />
    </main>
  );
}
