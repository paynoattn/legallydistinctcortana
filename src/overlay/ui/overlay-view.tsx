import { type ReactElement, useSyncExternalStore } from 'react';
import type { OverlayStore } from '../application/overlay-store.ts';
import { ErrorPill } from './error-pill.tsx';
import { Mark } from './mark.tsx';
import './overlay.css';

interface OverlayViewProps {
  readonly store: OverlayStore;
}

export function OverlayView({ store }: OverlayViewProps): ReactElement {
  const { state, level, reduceMotion } = useSyncExternalStore(store.subscribe, store.getSnapshot);
  return (
    <div className={reduceMotion ? 'overlay reduce-motion' : 'overlay'} data-testid="overlay">
      <Mark state={state.kind} level={level} reduceMotion={reduceMotion} />
      {state.kind === 'error' ? (
        <ErrorPill
          reason={state.reason}
          fix={state.fix}
          onFix={store.fix}
          onDismiss={store.dismiss}
        />
      ) : null}
    </div>
  );
}
