import type { ReactElement } from 'react';
import { markPose } from '../domain/mark-pose.ts';
import type { OverlayKind } from '../domain/overlay-state.ts';

interface MarkProps {
  readonly state: OverlayKind;
  readonly level: number;
  readonly reduceMotion: boolean;
}

function Lantern(): ReactElement {
  return (
    <>
      <path d="M24 1.5V7" stroke="#D9820B" strokeWidth="2.5" strokeLinecap="round" />
      <rect
        x="16.5"
        y="6.75"
        width="15"
        height="5.5"
        rx="1.75"
        fill="#D9820B"
        stroke="#1F2347"
        strokeWidth="2"
      />
      <rect
        x="18.5"
        y="37.75"
        width="11"
        height="5.5"
        rx="1.75"
        fill="#D9820B"
        stroke="#1F2347"
        strokeWidth="2"
      />
      <ellipse cx="24" cy="25" rx="18" ry="14" fill="#FFB81C" stroke="#1F2347" strokeWidth="2.5" />
    </>
  );
}

function Ribs(): ReactElement {
  return (
    <path
      d="M8.5 19.25Q24 22 39.5 19.25M8.5 30.75Q24 33.5 39.5 30.75"
      fill="none"
      stroke="#C76E00"
      strokeWidth="1.6"
      strokeLinecap="round"
      opacity=".8"
    />
  );
}

function CheckBadge(): ReactElement {
  return (
    <div className="lp-check">
      <svg viewBox="0 0 16 16">
        <path
          d="M3.5 8.5l3 3 6-7"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    </div>
  );
}

export function Mark({ state, level, reduceMotion }: MarkProps): ReactElement {
  const pose = markPose(state, level, reduceMotion);
  const coreTransform = pose.coreScale === null ? undefined : `scale(${String(pose.coreScale)})`;
  return (
    <div className="lp" data-testid="mark" data-state={state} aria-hidden="true">
      <div className="lp-halo" data-testid="halo" style={{ opacity: pose.halo }} />
      {state === 'speaking' ? <div className="lp-ring" data-testid="ring" /> : null}
      <svg className="lp-lantern" viewBox="0 0 48 48" focusable="false">
        <g className="lp-swing">
          <Lantern />
          <ellipse
            className="lp-core"
            data-testid="core"
            cx="24"
            cy="25.5"
            rx="8.5"
            ry="7"
            fill="#FFF1C7"
            style={{ opacity: pose.coreOpacity, transform: coreTransform }}
          />
          <Ribs />
        </g>
      </svg>
      <CheckBadge />
    </div>
  );
}
