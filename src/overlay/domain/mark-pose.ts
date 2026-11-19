import type { OverlayKind } from './overlay-state.ts';

export interface MarkPose {
  readonly coreScale: number | null;
  readonly coreOpacity: number;
  readonly halo: number;
}

const LEVEL_SCALE = 0.35;
const LISTENING_HALO = 0.3;
const LEVEL_HALO = 0.7;
const REDUCED_CORE = 0.78;
const REDUCED_LEVEL_CORE = 0.22;

const RESTING: Readonly<Record<Exclude<OverlayKind, 'listening'>, MarkPose>> = {
  hidden: { coreScale: null, coreOpacity: 1, halo: 0 },
  armed: { coreScale: null, coreOpacity: 1, halo: 0.22 },
  thinking: { coreScale: null, coreOpacity: 0.7, halo: 0.18 },
  inserted: { coreScale: null, coreOpacity: 1, halo: 0.3 },
  speaking: { coreScale: null, coreOpacity: 1, halo: 0.45 },
  error: { coreScale: null, coreOpacity: 0.45, halo: 0 },
};

export function clampLevel(level: number): number {
  return Math.min(1, Math.max(0, level));
}

function listening(level: number, reduceMotion: boolean): MarkPose {
  const halo = LISTENING_HALO + LEVEL_HALO * level;
  return reduceMotion
    ? { coreScale: null, coreOpacity: REDUCED_CORE + REDUCED_LEVEL_CORE * level, halo }
    : { coreScale: 1 + LEVEL_SCALE * level, coreOpacity: 1, halo };
}

export function markPose(kind: OverlayKind, level: number, reduceMotion: boolean): MarkPose {
  return kind === 'listening' ? listening(level, reduceMotion) : RESTING[kind];
}
