import { describe, expect, it } from 'vitest';
import type { Voice, VoiceChoice } from '../domain/voice.ts';
import type { VoiceCatalog, VoicePreferences } from './ports.ts';
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
const THORSTEN: Voice = {
  name: 'de_DE-thorsten-medium',
  description: 'thorsten (medium)',
  languages: ['de_DE'],
  speakers: [],
};
const VOICES = [AMY, LIBRITTS, THORSTEN];

class ScriptedCatalog implements VoiceCatalog {
  requests = 0;
  gate = Promise.withResolvers<readonly Voice[]>();

  listVoices(): Promise<readonly Voice[]> {
    this.requests += 1;
    return this.gate.promise;
  }

  serve(voices: readonly Voice[]): void {
    this.gate.resolve(voices);
  }

  fail(failure: unknown): void {
    this.gate.reject(failure);
  }

  next(): void {
    this.gate = Promise.withResolvers<readonly Voice[]>();
  }
}

class MemoryPreferences implements VoicePreferences {
  readonly saved: (VoiceChoice | null)[] = [];

  stored: VoiceChoice | null;

  constructor(stored: VoiceChoice | null) {
    this.stored = stored;
  }

  load(): VoiceChoice | null {
    return this.stored;
  }

  save(choice: VoiceChoice | null): void {
    this.stored = choice;
    this.saved.push(choice);
  }
}

interface Started {
  readonly catalog: ScriptedCatalog;
  readonly preferences: MemoryPreferences;
  readonly store: VoiceStore;
}

function start(stored: VoiceChoice | null = null, preferredLanguage = 'en-US'): Started {
  const catalog = new ScriptedCatalog();
  const preferences = new MemoryPreferences(stored);
  return {
    catalog,
    preferences,
    store: new VoiceStore({ catalog, preferences, preferredLanguage }),
  };
}

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

async function ready(
  stored: VoiceChoice | null = null,
  voices: readonly Voice[] = VOICES,
  preferredLanguage = 'en-US',
): Promise<Started> {
  const started = start(stored, preferredLanguage);
  started.store.load();
  started.catalog.serve(voices);
  await flush();
  return started;
}

async function readyUnsaved(...args: Parameters<typeof ready>): Promise<Started> {
  const started = await ready(...args);
  started.preferences.saved.length = 0;
  return started;
}

describe('VoiceStore loading', () => {
  it('starts loading, with no language and the saved choice', () => {
    const saved = { name: AMY.name, speaker: null };
    const { store } = start(saved);
    expect(store.getSnapshot()).toEqual({
      list: { kind: 'loading' },
      language: null,
      choice: saved,
    });
    expect(start().store.getSnapshot().choice).toBeNull();
  });

  it('asks the catalog only when told to load', () => {
    const { catalog, store } = start();
    expect(catalog.requests).toBe(0);
    store.load();
    expect(catalog.requests).toBe(1);
  });

  it('becomes ready with the voices and the preferred language', async () => {
    const { store } = await ready();
    expect(store.getSnapshot()).toEqual({
      list: { kind: 'ready', voices: VOICES },
      language: 'en_US',
      choice: null,
    });
  });

  it('falls back to a language that is offered when the preferred one is not', async () => {
    const { store } = await ready(null, [THORSTEN], 'en-US');
    expect(store.getSnapshot().language).toBe('de_DE');
  });

  it('has no language when there are no voices', async () => {
    const { store } = await ready(null, []);
    expect(store.getSnapshot()).toEqual({
      list: { kind: 'ready', voices: [] },
      language: null,
      choice: null,
    });
  });

  it('shows loading while voices are being fetched', () => {
    const { store } = start();
    store.load();
    expect(store.getSnapshot().list).toEqual({ kind: 'loading' });
  });

  it.each([
    ['an Error', new Error('no voice server'), 'no voice server'],
    ['a plain string', 'voice server is down', 'voice server is down'],
  ])('fails with the message when the catalog rejects with %s', async (_name, failure, text) => {
    const { catalog, store } = start();
    store.load();
    catalog.fail(failure);
    await flush();
    expect(store.getSnapshot().list).toEqual({ kind: 'failed', message: text });
  });

  it('keeps the saved choice when loading fails', async () => {
    const saved = { name: THORSTEN.name, speaker: null };
    const { catalog, store } = start(saved);
    store.load();
    catalog.fail('down');
    await flush();
    expect(store.getSnapshot().choice).toEqual(saved);
  });

  it('loads again after a failure', async () => {
    const { catalog, store } = start();
    store.load();
    catalog.fail('down');
    await flush();
    catalog.next();
    store.load();
    expect(store.getSnapshot().list).toEqual({ kind: 'loading' });
    catalog.serve(VOICES);
    await flush();
    expect(catalog.requests).toBe(2);
    expect(store.getSnapshot().list).toEqual({ kind: 'ready', voices: VOICES });
  });
});

describe('VoiceStore saved choice', () => {
  it('keeps a saved choice that is offered and shows its language', async () => {
    const saved = { name: THORSTEN.name, speaker: null };
    const { store } = await ready(saved);
    expect(store.getSnapshot().choice).toEqual(saved);
    expect(store.getSnapshot().language).toBe('de_DE');
  });

  it('keeps a saved speaker that the voice offers', async () => {
    const saved = { name: LIBRITTS.name, speaker: 'p240' };
    const { store } = await ready(saved);
    expect(store.getSnapshot().choice).toEqual(saved);
    expect(store.getSnapshot().language).toBe('en_US');
  });

  it.each<[string, VoiceChoice]>([
    ['a voice that is no longer offered', { name: 'gone', speaker: null }],
    ['a speaker the voice does not have', { name: LIBRITTS.name, speaker: 'p999' }],
  ])('drops a saved choice of %s and uses the preferred language', async (_name, saved) => {
    const { store } = await ready(saved, VOICES, 'de');
    expect(store.getSnapshot().choice).toBeNull();
    expect(store.current()).toBeNull();
    expect(store.getSnapshot().language).toBe('de_DE');
  });

  it('uses the language of the saved choice over the preferred language', async () => {
    const { store } = await ready({ name: THORSTEN.name, speaker: null }, VOICES, 'en-US');
    expect(store.getSnapshot().language).toBe('de_DE');
  });
});

describe('VoiceStore choosing', () => {
  it('picks the first voice of a language when the language is chosen', async () => {
    const { preferences, store } = await readyUnsaved();
    store.chooseLanguage('de_DE');
    expect(store.getSnapshot().language).toBe('de_DE');
    expect(store.current()).toEqual({ name: THORSTEN.name, speaker: null });
    expect(preferences.saved).toEqual([{ name: THORSTEN.name, speaker: null }]);
  });

  it('picks the first speaker of that first voice', async () => {
    const { preferences, store } = await readyUnsaved(null, [LIBRITTS, AMY, THORSTEN], 'de');
    store.chooseLanguage('en_US');
    expect(store.current()).toEqual({ name: LIBRITTS.name, speaker: 'p239' });
    expect(preferences.saved).toEqual([{ name: LIBRITTS.name, speaker: 'p239' }]);
  });

  it('chooses no voice for a language nobody speaks', async () => {
    const { preferences, store } = await readyUnsaved({ name: AMY.name, speaker: null });
    store.chooseLanguage('fr_FR');
    expect(store.getSnapshot().language).toBe('fr_FR');
    expect(store.current()).toBeNull();
    expect(preferences.saved).toEqual([null]);
  });

  it('chooses a voice and its first speaker, keeping the language', async () => {
    const { preferences, store } = await readyUnsaved();
    store.chooseVoice(LIBRITTS.name);
    expect(store.getSnapshot().language).toBe('en_US');
    expect(store.current()).toEqual({ name: LIBRITTS.name, speaker: 'p239' });
    store.chooseVoice(AMY.name);
    expect(store.current()).toEqual({ name: AMY.name, speaker: null });
    expect(preferences.saved).toEqual([
      { name: LIBRITTS.name, speaker: 'p239' },
      { name: AMY.name, speaker: null },
    ]);
  });

  it('means the server default when the voice is blank', async () => {
    const { preferences, store } = await readyUnsaved({ name: AMY.name, speaker: null });
    store.chooseVoice('');
    expect(store.current()).toBeNull();
    expect(store.getSnapshot().language).toBe('en_US');
    expect(preferences.saved).toEqual([null]);
  });

  it('means the server default for a voice nobody offers', async () => {
    const { store } = await readyUnsaved({ name: AMY.name, speaker: null });
    store.chooseVoice('gone');
    expect(store.current()).toBeNull();
  });

  it('chooses no voice before the voices are loaded', () => {
    const { preferences, store } = start({ name: AMY.name, speaker: null });
    store.chooseVoice(AMY.name);
    store.chooseLanguage('en_US');
    expect(store.current()).toBeNull();
    expect(preferences.saved).toEqual([null, null]);
  });

  it('chooses a speaker of the chosen voice, keeping the language', async () => {
    const { preferences, store } = await readyUnsaved({ name: LIBRITTS.name, speaker: 'p239' });
    store.chooseSpeaker('p240');
    expect(store.current()).toEqual({ name: LIBRITTS.name, speaker: 'p240' });
    expect(store.getSnapshot().language).toBe('en_US');
    expect(preferences.saved).toEqual([{ name: LIBRITTS.name, speaker: 'p240' }]);
  });

  it('means the default speaker when the speaker is blank, and still chooses a named one', async () => {
    const { preferences, store } = await readyUnsaved({ name: LIBRITTS.name, speaker: 'p240' });
    store.chooseSpeaker('');
    expect(store.current()).toEqual({ name: LIBRITTS.name, speaker: null });
    expect(store.getSnapshot().language).toBe('en_US');
    store.chooseSpeaker('p239');
    expect(store.current()).toEqual({ name: LIBRITTS.name, speaker: 'p239' });
    expect(preferences.saved).toEqual([
      { name: LIBRITTS.name, speaker: null },
      { name: LIBRITTS.name, speaker: 'p239' },
    ]);
  });

  it('ignores a speaker when no voice is chosen', async () => {
    const { preferences, store } = await readyUnsaved();
    let notified = 0;
    store.subscribe(() => {
      notified += 1;
    });
    const before = store.getSnapshot();
    store.chooseSpeaker('p240');
    expect(store.getSnapshot()).toBe(before);
    expect(store.current()).toBeNull();
    expect(preferences.saved).toEqual([]);
    expect(notified).toBe(0);
  });

  it('reports the current choice', async () => {
    const { store } = await readyUnsaved();
    expect(store.current()).toBeNull();
    store.chooseVoice(AMY.name);
    expect(store.current()).toEqual({ name: AMY.name, speaker: null });
  });
});

describe('VoiceStore subscriptions', () => {
  it('notifies subscribers on every change until they unsubscribe', async () => {
    const { catalog, store } = start();
    let notified = 0;
    const unsubscribe = store.subscribe(() => {
      notified += 1;
    });
    store.load();
    expect(notified).toBe(1);
    catalog.serve(VOICES);
    await flush();
    expect(notified).toBe(2);
    store.chooseVoice(AMY.name);
    expect(notified).toBe(3);
    unsubscribe();
    store.chooseVoice('');
    expect(notified).toBe(3);
  });

  it('keeps the snapshot identity stable between changes', async () => {
    const { store } = await ready();
    expect(store.getSnapshot()).toBe(store.getSnapshot());
  });
});
