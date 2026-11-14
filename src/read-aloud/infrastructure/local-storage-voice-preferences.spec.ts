import { beforeEach, describe, expect, it } from 'vitest';
import { LocalStorageVoicePreferences, VOICE_KEY } from './local-storage-voice-preferences.ts';

describe('LocalStorageVoicePreferences', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('loads nothing when nothing was saved', () => {
    expect(new LocalStorageVoicePreferences(window.localStorage).load()).toBeNull();
  });

  it.each([
    { name: 'en_US-amy-medium', speaker: null },
    { name: 'en_US-libritts-high', speaker: 'p239' },
  ])('round trips %j', (choice) => {
    new LocalStorageVoicePreferences(window.localStorage).save(choice);
    expect(new LocalStorageVoicePreferences(window.localStorage).load()).toEqual(choice);
  });

  it('keeps the choice under its own key', () => {
    new LocalStorageVoicePreferences(window.localStorage).save({ name: 'a', speaker: null });
    expect(VOICE_KEY).toBe('read-aloud.voice.v1');
    expect(window.localStorage.getItem(VOICE_KEY)).toBe('{"name":"a","speaker":null}');
  });

  it('removes the key when the choice is cleared', () => {
    const preferences = new LocalStorageVoicePreferences(window.localStorage);
    preferences.save({ name: 'a', speaker: null });
    preferences.save(null);
    expect(window.localStorage.getItem(VOICE_KEY)).toBeNull();
    expect(preferences.load()).toBeNull();
  });

  it.each([
    'not json',
    '',
    'null',
    '"text"',
    '[]',
    '{}',
    '{"name":"","speaker":null}',
    '{"name":"a"}',
    '{"name":"a","speaker":3}',
    '{"name":3,"speaker":null}',
  ])('loads %j as nothing', (raw) => {
    window.localStorage.setItem(VOICE_KEY, raw);
    expect(new LocalStorageVoicePreferences(window.localStorage).load()).toBeNull();
  });
});
