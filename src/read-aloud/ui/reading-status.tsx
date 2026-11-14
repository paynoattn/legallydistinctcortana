import type { ReactElement } from 'react';
import { moodLabel } from '../domain/mood.ts';
import type { ReadingStatus } from '../domain/read-aloud-state.ts';
import { onEscape } from './escape-key.ts';
import { focusFirstMenuItem } from './focus-on-mount.ts';

interface ReadingStatusProps {
  readonly status: ReadingStatus;
  readonly onStop: () => void;
  readonly onDismiss: () => void;
}

export function ReadingStatusLine({
  status,
  onStop,
  onDismiss,
}: ReadingStatusProps): ReactElement | null {
  switch (status.kind) {
    case 'idle':
      return null;
    case 'reading':
      return (
        <p className="reading-status" role="status">
          Reading in a {moodLabel(status.mood).toLowerCase()} voice.{' '}
          <button
            type="button"
            onClick={() => {
              onStop();
              focusFirstMenuItem();
            }}
            onKeyDown={onEscape(onDismiss)}
          >
            Stop
          </button>
        </p>
      );
    case 'finished':
      return <p role="status">Finished reading.</p>;
    case 'stopped':
      return <p role="status">Stopped.</p>;
    case 'failed':
      return <p role="alert">{status.message}</p>;
  }
}
