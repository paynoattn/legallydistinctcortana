import { describe, expect, it } from 'vitest';
import {
  LocalStorageReaderPreferences,
  MOOD_KEY,
  SPEED_KEY,
  TEXT_SIZE_KEY,
} from './local-storage-reader-preferences.ts';

function preferences(): LocalStorageReaderPreferences {
  return new LocalStorageReaderPreferences(window.localStorage);
}

describe('LocalStorageReaderPreferences', () => {
  it('uses versioned keys', () => {
    expect(MOOD_KEY).toBe('read-aloud.mood.v1');
    expect(TEXT_SIZE_KEY).toBe('read-aloud.text-size.v1');
    expect(SPEED_KEY).toBe('read-aloud.speed.v1');
  });

  it('has no mood or text size until one is saved', () => {
    expect(preferences().loadMood()).toBeNull();
    expect(preferences().loadTextSize()).toBeNull();
  });

  it('remembers the mood', () => {
    preferences().saveMood('whisper');
    expect(window.localStorage.getItem(MOOD_KEY)).toBe('whisper');
    expect(preferences().loadMood()).toBe('whisper');
  });

  it('hands back a stored mood as it is, for the store to check', () => {
    window.localStorage.setItem(MOOD_KEY, 'sad');
    expect(preferences().loadMood()).toBe('sad');
  });

  it('remembers the text size', () => {
    preferences().saveTextSize(31);
    expect(window.localStorage.getItem(TEXT_SIZE_KEY)).toBe('31');
    expect(preferences().loadTextSize()).toBe('31');
  });

  it('hands back a stored text size as it is, for the store to check', () => {
    window.localStorage.setItem(TEXT_SIZE_KEY, 'big');
    expect(preferences().loadTextSize()).toBe('big');
  });

  it('remembers the speed, and has none until one is saved', () => {
    expect(preferences().loadSpeed()).toBeNull();
    preferences().saveSpeed(1500);
    expect(window.localStorage.getItem(SPEED_KEY)).toBe('1500');
    expect(preferences().loadSpeed()).toBe('1500');
  });
});
