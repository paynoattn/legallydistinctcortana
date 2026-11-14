import { beforeEach, describe, expect, it } from 'vitest';
import type { Voice, VoiceChoice } from '../domain/voice.ts';
import {
  LocalStorageVoicePreferences,
  VOICE_KEY,
} from '../infrastructure/local-storage-voice-preferences.ts';
import type { VoiceCatalog } from './ports.ts';
import { VoiceStore } from './voice-store.ts';

const AMY: Voice = {
  name: 'en_US-amy-medium',
  description: 'amy (medium)',
  languages: ['en_US'],
  speakers: [],
};
const LIBRITTS: Voice = {
  name: 'en_US-libritts-high',
  description: 'libritts (high)',
  languages: ['en_US'],
  speakers: ['p239', 'p240'],
};
const VOICES = [AMY, LIBRITTS];

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

function catalogOf(outcome: Promise<readonly Voice[]>): VoiceCatalog {
  return { listVoices: () => outcome };
}

function storeOver(catalog: VoiceCatalog): VoiceStore {
  return new VoiceStore({
    catalog,
    preferences: new LocalStorageVoicePreferences(window.localStorage),
    preferredLanguage: 'en-US',
  });
}

async function loaded(choice: VoiceChoice): Promise<VoiceStore> {
  window.localStorage.setItem(VOICE_KEY, JSON.stringify(choice));
  const store = storeOver(catalogOf(Promise.resolve(VOICES)));
  store.load();
  await flush();
  return store;
}

describe('VoiceStore reconciling the saved choice into storage', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it.each<[string, VoiceChoice]>([
    ['a voice the server no longer offers', { name: 'gone', speaker: null }],
    ['a speaker the voice does not have', { name: LIBRITTS.name, speaker: 'p999' }],
  ])('removes a saved choice of %s from storage', async (_name, saved) => {
    const store = await loaded(saved);
    expect(store.current()).toBeNull();
    expect(window.localStorage.getItem(VOICE_KEY)).toBeNull();
  });

  it('does not restore a rejected choice after a later failed discovery', async () => {
    await loaded({ name: 'gone', speaker: null });
    const later = storeOver(catalogOf(Promise.reject(new Error('down'))));
    later.load();
    await flush();
    expect(later.getSnapshot().list.kind).toBe('failed');
    expect(later.current()).toBeNull();
  });

  it.each<VoiceChoice>([
    { name: AMY.name, speaker: null },
    { name: LIBRITTS.name, speaker: 'p240' },
  ])('keeps a saved choice that is still offered, unchanged: %j', async (saved) => {
    const store = await loaded(saved);
    expect(store.current()).toEqual(saved);
    expect(window.localStorage.getItem(VOICE_KEY)).toBe(JSON.stringify(saved));
  });

  it('leaves storage alone when the voices fail to load', async () => {
    const saved = { name: 'gone', speaker: null };
    window.localStorage.setItem(VOICE_KEY, JSON.stringify(saved));
    const store = storeOver(catalogOf(Promise.reject(new Error('down'))));
    store.load();
    await flush();
    expect(window.localStorage.getItem(VOICE_KEY)).toBe(JSON.stringify(saved));
  });
});
