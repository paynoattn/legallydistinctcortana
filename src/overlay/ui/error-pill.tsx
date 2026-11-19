import type { ReactElement } from 'react';
import { type Fix, fixLabel } from '../domain/overlay-state.ts';

interface ErrorPillProps {
  readonly reason: string;
  readonly fix: Fix | null;
  readonly onFix: () => void;
  readonly onDismiss: () => void;
}

export function ErrorPill({ reason, fix, onFix, onDismiss }: ErrorPillProps): ReactElement {
  return (
    <div className="err">
      <button type="button" className="err-dismiss" onClick={onDismiss}>
        {reason}
      </button>
      {fix === null ? null : (
        <button type="button" className="err-fix" onClick={onFix}>
          {fixLabel(fix)}
        </button>
      )}
    </div>
  );
}
