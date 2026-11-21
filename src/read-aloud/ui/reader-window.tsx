import { type ReactElement, useSyncExternalStore } from 'react';
import type { ReaderStore } from '../application/reader-store.ts';
import type { VoiceStore } from '../application/voice-store.ts';
import type { Selection } from '../domain/read-aloud-state.ts';
import { ControlBar } from './control-bar.tsx';
import { focusOnMount } from './focus-on-mount.ts';
import { readerKeys } from './reader-keyboard.ts';
import { ReaderText } from './reader-text.tsx';
import './read-aloud.css';

interface ReaderWindowProps {
  readonly store: ReaderStore;
  readonly voices: VoiceStore;
}

interface TitleRowProps {
  readonly focusClose: boolean;
  readonly onClose: () => void;
}

interface SelectionPromptProps {
  readonly selection: Exclude<Selection, { kind: 'ready' }>;
}

function TitleRow({ focusClose, onClose }: TitleRowProps): ReactElement {
  return (
    <header className="reader-title">
      <span className="reader-mark" aria-hidden="true" />
      <h1 id="reader-title">Read aloud</h1>
      <button
        type="button"
        className="reader-close"
        aria-label="Close"
        ref={focusClose ? focusOnMount : undefined}
        onClick={onClose}
      />
    </header>
  );
}

function SelectionPrompt({ selection }: SelectionPromptProps): ReactElement {
  return (
    <div className="reader-message">
      {selection.kind === 'failed' ? (
        <p role="alert">{selection.message}</p>
      ) : (
        <p>Select text in any app, then press the read-aloud shortcut.</p>
      )}
    </div>
  );
}

export function ReaderWindow({ store, voices }: ReaderWindowProps): ReactElement {
  const snapshot = useSyncExternalStore(store.subscribe, store.getSnapshot);
  const { selection, round } = snapshot;
  return (
    <main className="reader" aria-labelledby="reader-title" ref={readerKeys(store)}>
      <TitleRow key={round} focusClose={selection.kind !== 'ready'} onClose={store.close} />
      {selection.kind === 'ready' ? (
        <>
          <ReaderText
            store={store}
            sentences={selection.sentences}
            sentence={snapshot.sentence}
            word={snapshot.word}
            textSize={snapshot.textSize}
            following={snapshot.following}
          />
          <ControlBar
            key={round}
            store={store}
            voices={voices}
            status={snapshot.status}
            sentence={snapshot.sentence}
            count={selection.sentences.length}
            mood={snapshot.mood}
            speed={snapshot.speed}
            popover={snapshot.popover}
          />
        </>
      ) : (
        <SelectionPrompt selection={selection} />
      )}
    </main>
  );
}
