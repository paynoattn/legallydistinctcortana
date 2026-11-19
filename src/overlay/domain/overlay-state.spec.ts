import { describe, expect, it } from 'vitest';
import { failed, fixLabel, HIDDEN } from './overlay-state.ts';

describe('overlay state', () => {
  it('starts hidden', () => {
    expect(HIDDEN).toEqual({ kind: 'hidden' });
  });

  it('labels the sound settings fix', () => {
    expect(fixLabel('sound_settings')).toBe('Choose a mic');
  });

  it('labels the settings fix', () => {
    expect(fixLabel('settings')).toBe('Open settings');
  });

  it('turns a failure into an error without a fix and with motion allowed', () => {
    expect(failed('Broken.')).toEqual({
      state: { kind: 'error', reason: 'Broken.', fix: null },
      reduceMotion: false,
    });
  });
});
