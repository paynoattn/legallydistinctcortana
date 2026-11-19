import { describe, expect, it } from 'vitest';
import { clampLevel, markPose } from './mark-pose.ts';
import type { OverlayKind } from './overlay-state.ts';

describe('clampLevel', () => {
  it.each([
    [-0.5, 0],
    [0, 0],
    [0.4, 0.4],
    [1, 1],
    [1.7, 1],
  ])('clamps %d to %d', (level, clamped) => {
    expect(clampLevel(level)).toBe(clamped);
  });
});

describe('markPose while listening', () => {
  it('rests at the base size and halo when silent', () => {
    expect(markPose('listening', 0, false)).toEqual({ coreScale: 1, coreOpacity: 1, halo: 0.3 });
  });

  it('grows the core by a third and fills the halo at full level', () => {
    expect(markPose('listening', 1, false)).toEqual({ coreScale: 1.35, coreOpacity: 1, halo: 1 });
  });

  it('follows the level in between', () => {
    const pose = markPose('listening', 0.5, false);
    expect(pose.coreScale).toBeCloseTo(1.175);
    expect(pose.halo).toBeCloseTo(0.65);
  });

  it('keeps the core still under reduced motion and shows the level as brightness', () => {
    expect(markPose('listening', 0, true)).toEqual({
      coreScale: null,
      coreOpacity: 0.78,
      halo: 0.3,
    });
    expect(markPose('listening', 1, true)).toEqual({ coreScale: null, coreOpacity: 1, halo: 1 });
  });
});

const RESTING_POSES: readonly (readonly [OverlayKind, number, number])[] = [
  ['hidden', 1, 0],
  ['armed', 1, 0.22],
  ['thinking', 0.7, 0.18],
  ['inserted', 1, 0.3],
  ['speaking', 1, 0.45],
  ['error', 0.45, 0],
];

describe('markPose in the other states', () => {
  it.each(RESTING_POSES)('poses %s with core opacity %d and halo %d', (kind, coreOpacity, halo) => {
    expect(markPose(kind, 1, false)).toEqual({ coreScale: null, coreOpacity, halo });
  });
});
