import { describe, expect, it, vi } from 'vitest';
import type { Connection } from '../domain/connection.ts';
import { HOST_MESSAGE, type Settings, toDraft } from '../domain/settings.ts';
import type { SaveResult, SettingsGateway } from './ports.ts';
import { SettingsStore } from './settings-store.ts';

function rejected<T>(reason: unknown): Promise<T> {
  const { promise, reject } = Promise.withResolvers<T>();
  reject(reason);
  return promise;
}

const SETTINGS: Settings = {
  dictateShortcut: 'F9',
  readAloudShortcut: 'Ctrl+Alt+R',
  voiceHost: '127.0.0.1',
  sttPort: 10_300,
  ttsPort: 10_200,
  sttLanguage: 'auto',
  reduceMotion: false,
  showOverlay: true,
  startAtSignIn: false,
};

const DEFAULTS: Settings = { ...SETTINGS, dictateShortcut: 'F2', readAloudShortcut: 'F3' };
const CONNECTED: Connection = { kind: 'connected', summary: 'All good.' };

function outcome<T>(value: T, failure: unknown): Promise<T> {
  return failure === undefined ? Promise.resolve(value) : rejected(failure);
}

class ScriptedGateway implements SettingsGateway {
  readonly calls: string[] = [];
  readonly saved: Settings[] = [];
  loadFailure: unknown = undefined;
  defaultsFailure: unknown = undefined;
  saveResult: SaveResult | undefined = undefined;
  saveFailure: unknown = undefined;
  saveGate: Promise<void> = Promise.resolve();
  statusFailure: unknown = undefined;
  statusGate: Promise<void> = Promise.resolve();
  checkResult: Connection = CONNECTED;
  checkFailure: unknown = undefined;
  shortcutFailure: unknown = undefined;
  emit: (connection: Connection) => void = () => undefined;

  load(): Promise<Settings> {
    return outcome(SETTINGS, this.loadFailure);
  }

  defaults(): Promise<Settings> {
    return outcome(DEFAULTS, this.defaultsFailure);
  }

  async save(settings: Settings): Promise<SaveResult> {
    this.saved.push(settings);
    await this.saveGate;
    return outcome(this.saveResult ?? { kind: 'saved', settings }, this.saveFailure);
  }

  async connectionStatus(): Promise<Connection> {
    await this.statusGate;
    return outcome(CONNECTED, this.statusFailure);
  }

  checkConnection(): Promise<Connection> {
    this.calls.push('check');
    return outcome(this.checkResult, this.checkFailure);
  }

  onConnection(listener: (connection: Connection) => void): void {
    this.emit = listener;
  }

  suspendShortcuts(): Promise<void> {
    this.calls.push('suspend');
    return outcome(undefined, this.shortcutFailure);
  }

  resumeShortcuts(): Promise<void> {
    this.calls.push('resume');
    return outcome(undefined, this.shortcutFailure);
  }
}

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

async function loaded(gateway = new ScriptedGateway()): Promise<{
  gateway: ScriptedGateway;
  store: SettingsStore;
}> {
  const store = new SettingsStore(gateway);
  store.load();
  await flush();
  return { gateway, store };
}

function draftOf(store: SettingsStore): unknown {
  const { form } = store.getSnapshot();
  return form.kind === 'ready' ? form.draft : form.kind;
}

describe('SettingsStore loading', () => {
  it('starts loading with nothing else going on', () => {
    expect(new SettingsStore(new ScriptedGateway()).getSnapshot()).toEqual({
      form: { kind: 'loading' },
      coreErrors: {},
      notice: { kind: 'none' },
      saving: false,
      recording: null,
      connection: null,
      checking: false,
    });
  });

  it('shows the loaded settings as both saved and draft', async () => {
    const { store } = await loaded();
    expect(store.getSnapshot().form).toEqual({
      kind: 'ready',
      saved: toDraft(SETTINGS),
      draft: toDraft(SETTINGS),
    });
  });

  it('shows why loading failed', async () => {
    const gateway = new ScriptedGateway();
    gateway.loadFailure = new Error('No core.');
    const { store } = await loaded(gateway);
    expect(store.getSnapshot().form).toEqual({ kind: 'failed', message: 'No core.' });
  });

  it('shows a failure that is not an Error as text', async () => {
    const gateway = new ScriptedGateway();
    gateway.loadFailure = 'denied';
    const { store } = await loaded(gateway);
    expect(store.getSnapshot().form).toEqual({ kind: 'failed', message: 'denied' });
  });

  it('notifies subscribers until they unsubscribe', async () => {
    const store = new SettingsStore(new ScriptedGateway());
    const listener = vi.fn<() => void>();
    const unsubscribe = store.subscribe(listener);
    store.load();
    await flush();
    unsubscribe();
    store.checkConnection();
    await flush();
    expect(listener).toHaveBeenCalledTimes(2);
  });
});

describe('SettingsStore editing', () => {
  it('ignores edits before the settings load', () => {
    const store = new SettingsStore(new ScriptedGateway());
    store.edit('voiceHost', 'elsewhere');
    expect(store.getSnapshot().form).toEqual({ kind: 'loading' });
  });

  it('changes only the draft', async () => {
    const { store } = await loaded();
    store.edit('voiceHost', 'elsewhere');
    expect(store.getSnapshot().form).toEqual({
      kind: 'ready',
      saved: toDraft(SETTINGS),
      draft: { ...toDraft(SETTINGS), voiceHost: 'elsewhere' },
    });
  });

  it('clears the core error of the edited field and the notice', async () => {
    const gateway = new ScriptedGateway();
    gateway.saveResult = {
      kind: 'rejected',
      errors: { sttLanguage: 'Unknown.', voiceHost: 'Unreachable.' },
    };
    const { store } = await loaded(gateway);
    store.edit('sttLanguage', 'xx');
    store.save();
    await flush();
    store.edit('sttLanguage', 'en');
    expect(store.getSnapshot()).toMatchObject({ notice: { kind: 'none' } });
    expect(store.getSnapshot().coreErrors).toEqual({ voiceHost: 'Unreachable.' });
  });

  it('reverts the draft to the saved settings and forgets core errors', async () => {
    const gateway = new ScriptedGateway();
    gateway.saveResult = { kind: 'rejected', errors: { voiceHost: 'Unreachable.' } };
    const { store } = await loaded(gateway);
    store.edit('voiceHost', 'elsewhere');
    store.save();
    await flush();
    store.revert();
    expect(store.getSnapshot()).toMatchObject({
      form: { draft: toDraft(SETTINGS) },
      coreErrors: {},
      notice: { kind: 'none' },
    });
  });

  it('ignores revert before the settings load', () => {
    const store = new SettingsStore(new ScriptedGateway());
    store.revert();
    expect(store.getSnapshot().form).toEqual({ kind: 'loading' });
  });

  it('resets a shortcut to its default', async () => {
    const { store } = await loaded();
    store.resetShortcut('readAloudShortcut');
    await flush();
    expect(draftOf(store)).toEqual({ ...toDraft(SETTINGS), readAloudShortcut: 'F3' });
  });

  it('shows a failed reset', async () => {
    const gateway = new ScriptedGateway();
    gateway.defaultsFailure = new Error('No defaults.');
    const { store } = await loaded(gateway);
    store.resetShortcut('dictateShortcut');
    await flush();
    expect(store.getSnapshot().notice).toEqual({ kind: 'failed', message: 'No defaults.' });
  });
});

describe('SettingsStore saving', () => {
  it('saves a valid draft and shows the normalized result', async () => {
    const gateway = new ScriptedGateway();
    gateway.saveResult = { kind: 'saved', settings: { ...SETTINGS, voiceHost: 'voice.local' } };
    const { store } = await loaded(gateway);
    store.edit('voiceHost', 'Voice.Local');
    store.edit('ttsPort', '10201');
    store.save();
    expect(store.getSnapshot()).toMatchObject({ saving: true, notice: { kind: 'none' } });
    await flush();
    expect(gateway.saved).toEqual([{ ...SETTINGS, voiceHost: 'Voice.Local', ttsPort: 10_201 }]);
    const normalized = toDraft({ ...SETTINGS, voiceHost: 'voice.local' });
    expect(store.getSnapshot()).toMatchObject({
      form: { kind: 'ready', saved: normalized, draft: normalized },
      coreErrors: {},
      notice: { kind: 'saved' },
      saving: false,
    });
  });

  it('does not save before the settings load', () => {
    const gateway = new ScriptedGateway();
    new SettingsStore(gateway).save();
    expect(gateway.saved).toEqual([]);
  });

  it('does not save an invalid draft', async () => {
    const { gateway, store } = await loaded();
    store.edit('voiceHost', '');
    store.save();
    await flush();
    expect(gateway.saved).toEqual([]);
    expect(store.getSnapshot().saving).toBe(false);
  });

  it('saves once while a save is running', async () => {
    const gateway = new ScriptedGateway();
    const gate = Promise.withResolvers<undefined>();
    gateway.saveGate = gate.promise;
    const { store } = await loaded(gateway);
    store.save();
    store.save();
    gate.resolve(undefined);
    await flush();
    expect(gateway.saved).toHaveLength(1);
  });

  it('shows the core field errors and keeps the draft', async () => {
    const gateway = new ScriptedGateway();
    gateway.saveResult = { kind: 'rejected', errors: { voiceHost: HOST_MESSAGE } };
    const { store } = await loaded(gateway);
    store.edit('voiceHost', 'bad.host');
    store.save();
    await flush();
    expect(store.getSnapshot()).toMatchObject({
      form: { draft: { voiceHost: 'bad.host' } },
      coreErrors: { voiceHost: HOST_MESSAGE },
      notice: { kind: 'none' },
      saving: false,
    });
  });

  it('shows a failed save', async () => {
    const gateway = new ScriptedGateway();
    gateway.saveFailure = new Error('Disk full.');
    const { store } = await loaded(gateway);
    store.save();
    await flush();
    expect(store.getSnapshot()).toMatchObject({
      notice: { kind: 'failed', message: 'Disk full.' },
      saving: false,
    });
  });
});
