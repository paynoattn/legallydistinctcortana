import { type ReactElement, useSyncExternalStore } from 'react';
import type { ReadAloudStore } from '../application/read-aloud-store.ts';
import { describeLength, type Selection } from '../domain/read-aloud-state.ts';
import { onEscape } from './escape-key.ts';
import { focusOnMount } from './focus-on-mount.ts';
import { MoodMenu } from './mood-menu.tsx';
import { ReadingStatusLine } from './reading-status.tsx';
import './read-aloud.css';

interface ReadAloudPanelProps {
  readonly store: ReadAloudStore;
}

interface CloseButtonProps {
  readonly onDismiss: () => void;
}

function CloseButton({ onDismiss }: CloseButtonProps): ReactElement {
  return (
    <button type="button" ref={focusOnMount} onClick={onDismiss} onKeyDown={onEscape(onDismiss)}>
      Close
    </button>
  );
}

interface SelectionPromptProps {
  readonly selection: Exclude<Selection, { kind: 'ready' }>;
  readonly onDismiss: () => void;
}

function SelectionPrompt({ selection, onDismiss }: SelectionPromptProps): ReactElement {
  return (
    <>
      {selection.kind === 'failed' ? (
        <p role="alert">{selection.message}</p>
      ) : (
        <p>Select text in any app, then press the read-aloud shortcut.</p>
      )}
      <CloseButton onDismiss={onDismiss} />
    </>
  );
}

export function ReadAloudPanel({ store }: ReadAloudPanelProps): ReactElement {
  const { selection, status, round } = useSyncExternalStore(store.subscribe, store.getSnapshot);
  return (
    <main className="read-aloud" aria-labelledby="read-aloud-title">
      <h1 id="read-aloud-title">Read aloud</h1>
      <ReadingStatusLine status={status} onStop={store.stop} onDismiss={store.dismiss} />
      {selection.kind === 'ready' ? (
        <>
          <figure>
            <blockquote>{selection.preview}</blockquote>
            <figcaption>{describeLength(selection.charCount)}</figcaption>
          </figure>
          <MoodMenu key={round} onChoose={store.choose} onDismiss={store.dismiss} />
        </>
      ) : (
        <SelectionPrompt key={round} selection={selection} onDismiss={store.dismiss} />
      )}
    </main>
  );
}
