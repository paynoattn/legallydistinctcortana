import { act, fireEvent, render, screen } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import type { VoiceCatalog, VoicePreferences } from '../application/ports.ts';
import { VoiceStore } from '../application/voice-store.ts';
import type { VoiceChoice } from '../domain/voice.ts';
import { FAKE_VOICES } from '../infrastructure/fake-read-aloud-gateway.ts';
import { VoicePicker } from './voice-picker.tsx';

class ScriptedCatalog implements VoiceCatalog {
  requests = 0;
  gate = Promise.withResolvers<typeof FAKE_VOICES>();

  listVoices(): Promise<typeof FAKE_VOICES> {
    this.requests += 1;
    return this.gate.promise;
  }
}

class MemoryPreferences implements VoicePreferences {
  stored: VoiceChoice | null;

  constructor(stored: VoiceChoice | null) {
    this.stored = stored;
  }

  load(): VoiceChoice | null {
    return this.stored;
  }

  save(choice: VoiceChoice | null): void {
    this.stored = choice;
  }
}

interface Started {
  readonly catalog: ScriptedCatalog;
  readonly store: VoiceStore;
  readonly dismissals: number[];
  readonly user: ReturnType<typeof userEvent.setup>;
}

function start(stored: VoiceChoice | null = null): Started {
  const catalog = new ScriptedCatalog();
  const store = new VoiceStore({
    catalog,
    preferences: new MemoryPreferences(stored),
    preferredLanguage: 'en-US',
  });
  const dismissals: number[] = [];
  render(<VoicePicker store={store} onDismiss={() => dismissals.push(dismissals.length)} />);
  return { catalog, store, dismissals, user: userEvent.setup() };
}

async function startReady(
  stored: VoiceChoice | null = null,
  voices: typeof FAKE_VOICES = FAKE_VOICES,
): Promise<Started> {
  const started = start(stored);
  await act(async () => {
    started.store.load();
    started.catalog.gate.resolve(voices);
    await started.catalog.gate.promise;
  });
  return started;
}

async function startFailed(message: string): Promise<Started> {
  const started = start();
  await act(async () => {
    started.store.load();
    started.catalog.gate.reject(new Error(message));
    await started.catalog.gate.promise.catch(() => undefined);
  });
  return started;
}

const language = (): HTMLSelectElement =>
  screen.getByRole<HTMLSelectElement>('combobox', { name: /^Language/ });
const voice = (): HTMLSelectElement =>
  screen.getByRole<HTMLSelectElement>('combobox', { name: /^Voice/ });
const speaker = (): HTMLSelectElement =>
  screen.getByRole<HTMLSelectElement>('combobox', { name: /^Speaker/ });
const optionTexts = (select: HTMLSelectElement): string[] =>
  [...select.options].map((option) => option.textContent);

describe('VoicePicker while loading', () => {
  it('says the voices are loading and offers nothing to choose', () => {
    start();
    expect(screen.getByText('Loading voices…')).toBeTruthy();
    expect(screen.queryByRole('combobox')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
  });
});

describe('VoicePicker when the voices are unavailable', () => {
  it('shows the reason as an alert with a Retry button', async () => {
    await startFailed('no voice server');
    expect(screen.getByRole('alert').textContent).toBe(
      'Voices are unavailable: no voice server Retry',
    );
    expect(screen.queryByRole('combobox')).toBeNull();
  });

  it('loads the voices again with Retry', async () => {
    const { catalog, user } = await startFailed('no voice server');
    catalog.gate = Promise.withResolvers<typeof FAKE_VOICES>();
    await user.click(screen.getByRole('button', { name: 'Retry' }));
    expect(catalog.requests).toBe(2);
    expect(screen.getByText('Loading voices…')).toBeTruthy();
    await act(async () => {
      catalog.gate.resolve(FAKE_VOICES);
      await catalog.gate.promise;
    });
    expect(screen.getAllByRole('combobox')).toHaveLength(2);
  });

  it('dismisses with Escape on Retry and ignores other keys', async () => {
    const { dismissals } = await startFailed('no voice server');
    const retry = screen.getByRole('button', { name: 'Retry' });
    fireEvent.keyDown(retry, { key: 'x' });
    expect(dismissals).toEqual([]);
    fireEvent.keyDown(retry, { key: 'Escape' });
    expect(dismissals).toHaveLength(1);
  });
});

describe('VoicePicker with voices', () => {
  it('shows the language and voice selects, labelled, in a Voice group', async () => {
    await startReady();
    expect(screen.getByRole('group', { name: 'Voice' })).toBeTruthy();
    expect(screen.getAllByRole('combobox')).toHaveLength(2);
    expect(language().value).toBe('en_US');
    expect(voice().value).toBe('');
    expect(screen.queryByRole('combobox', { name: /^Speaker/ })).toBeNull();
  });

  it('lists the languages and the voices of the language, the server default first', async () => {
    await startReady();
    expect(optionTexts(language())).toEqual([
      'German (Germany) (de_DE)',
      'American English (en_US)',
    ]);
    expect(optionTexts(voice())).toEqual(['Server default', 'amy (medium)', 'libritts (high)']);
    expect(voice().options[0]?.value).toBe('');
  });

  it('shows the saved voice, in its language', async () => {
    await startReady({ name: 'de_DE-thorsten-medium', speaker: null });
    expect(language().value).toBe('de_DE');
    expect(voice().value).toBe('de_DE-thorsten-medium');
    expect(optionTexts(voice())).toEqual(['Server default', 'thorsten (medium)']);
  });

  it('shows the speaker select for a multi-speaker voice only', async () => {
    const { user } = await startReady();
    await user.selectOptions(voice(), 'en_US-libritts-high');
    expect(optionTexts(speaker())).toEqual(['Default speaker', 'p239', 'p240']);
    expect(speaker().value).toBe('p239');
    await user.selectOptions(voice(), 'en_US-amy-medium');
    expect(screen.queryByRole('combobox', { name: /^Speaker/ })).toBeNull();
  });
});

describe('VoicePicker edge cases', () => {
  it('shows empty selects when the server offers no voices', async () => {
    await startReady(null, []);
    expect(language().value).toBe('');
    expect(language().options).toHaveLength(0);
    expect(optionTexts(voice())).toEqual(['Server default']);
  });

  it('shows the default speaker when a multi-speaker voice has none chosen yet', async () => {
    const { store } = await startReady({ name: 'en_US-libritts-high', speaker: null });
    expect(store.current()).toEqual({ name: 'en_US-libritts-high', speaker: null });
    expect(optionTexts(speaker())).toEqual(['Default speaker', 'p239', 'p240']);
    expect(speaker().value).toBe('');
    expect(speaker().selectedOptions[0]?.textContent).toBe('Default speaker');
  });
});

describe('VoicePicker choosing', () => {
  it('chooses a language, which picks its first voice', async () => {
    const { store, user } = await startReady();
    await user.selectOptions(language(), 'de_DE');
    expect(store.current()).toEqual({ name: 'de_DE-thorsten-medium', speaker: null });
    expect(language().value).toBe('de_DE');
    expect(voice().value).toBe('de_DE-thorsten-medium');
  });

  it('chooses a voice', async () => {
    const { store, user } = await startReady();
    await user.selectOptions(voice(), 'en_US-amy-medium');
    expect(store.current()).toEqual({ name: 'en_US-amy-medium', speaker: null });
    expect(voice().value).toBe('en_US-amy-medium');
  });

  it('goes back to the server default', async () => {
    const { store, user } = await startReady({ name: 'en_US-amy-medium', speaker: null });
    await user.selectOptions(voice(), '');
    expect(store.current()).toBeNull();
    expect(voice().value).toBe('');
  });

  it('chooses a speaker', async () => {
    const { store, user } = await startReady({ name: 'en_US-libritts-high', speaker: 'p239' });
    await user.selectOptions(speaker(), 'p240');
    expect(store.current()).toEqual({ name: 'en_US-libritts-high', speaker: 'p240' });
    expect(speaker().value).toBe('p240');
  });
});

describe('VoicePicker choosing the default speaker', () => {
  it('chooses the default speaker, then a named speaker again', async () => {
    const { store, user } = await startReady({ name: 'en_US-libritts-high', speaker: 'p240' });
    await user.selectOptions(speaker(), 'Default speaker');
    expect(store.current()).toEqual({ name: 'en_US-libritts-high', speaker: null });
    expect(speaker().value).toBe('');
    await user.selectOptions(speaker(), 'p239');
    expect(store.current()).toEqual({ name: 'en_US-libritts-high', speaker: 'p239' });
    expect(speaker().value).toBe('p239');
  });
});

describe('VoicePicker dismissing', () => {
  it.each([
    ['language', language],
    ['voice', voice],
    ['speaker', speaker],
  ])('dismisses with Escape on the %s select and ignores other keys', async (_name, select) => {
    const { dismissals } = await startReady({ name: 'en_US-libritts-high', speaker: 'p239' });
    fireEvent.keyDown(select(), { key: 'ArrowDown' });
    expect(dismissals).toEqual([]);
    fireEvent.keyDown(select(), { key: 'Escape' });
    expect(dismissals).toHaveLength(1);
  });
});
