import { describe, expect, it } from 'vitest';
import type { Connection } from '../domain/connection.ts';
import { type Settings, toDraft } from '../domain/settings.ts';
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

describe('SettingsStore connection', () => {
  it('shows the current connection after loading', async () => {
    const { store } = await loaded();
    expect(store.getSnapshot().connection).toEqual(CONNECTED);
  });

  it('keeps a newer pushed connection over the initial answer', async () => {
    const gateway = new ScriptedGateway();
    const gate = Promise.withResolvers<undefined>();
    gateway.statusGate = gate.promise;
    const store = new SettingsStore(gateway);
    store.load();
    const pushed: Connection = { kind: 'unreachable', summary: 'Gone.' };
    gateway.emit(pushed);
    gate.resolve(undefined);
    await flush();
    expect(store.getSnapshot().connection).toEqual(pushed);
  });

  it('shows a failed status request as an unknown connection', async () => {
    const gateway = new ScriptedGateway();
    gateway.statusFailure = new Error('No answer.');
    const { store } = await loaded(gateway);
    expect(store.getSnapshot().connection).toEqual({ kind: 'failed', summary: 'No answer.' });
  });

  it('marks a test as running until it answers', async () => {
    const { gateway, store } = await loaded();
    gateway.checkResult = { kind: 'partial', summary: 'Only voice.' };
    store.checkConnection();
    expect(store.getSnapshot().checking).toBe(true);
    await flush();
    expect(store.getSnapshot()).toMatchObject({
      checking: false,
      connection: { kind: 'partial', summary: 'Only voice.' },
    });
  });

  it('keeps testing while the core pushes progress', async () => {
    const { gateway, store } = await loaded();
    store.checkConnection();
    const checking: Connection = { kind: 'checking', summary: 'Probing.' };
    gateway.emit(checking);
    expect(store.getSnapshot()).toMatchObject({ checking: true, connection: checking });
    await flush();
    expect(store.getSnapshot()).toMatchObject({ checking: false, connection: CONNECTED });
  });

  it('shows a failed test', async () => {
    const { gateway, store } = await loaded();
    gateway.checkFailure = new Error('Timed out.');
    store.checkConnection();
    await flush();
    expect(store.getSnapshot()).toMatchObject({
      checking: false,
      connection: { kind: 'failed', summary: 'Timed out.' },
    });
  });
});

describe('SettingsStore recording', () => {
  it('suspends the global shortcuts while recording', async () => {
    const { gateway, store } = await loaded();
    store.startRecording('dictateShortcut');
    expect(store.getSnapshot().recording).toBe('dictateShortcut');
    expect(gateway.calls).toEqual(['suspend']);
  });

  it('suspends once when switching to the other shortcut', async () => {
    const { gateway, store } = await loaded();
    store.startRecording('dictateShortcut');
    store.startRecording('readAloudShortcut');
    expect(store.getSnapshot().recording).toBe('readAloudShortcut');
    expect(gateway.calls).toEqual(['suspend']);
  });

  it('records the chord into the draft and resumes the shortcuts', async () => {
    const { gateway, store } = await loaded();
    store.startRecording('readAloudShortcut');
    store.record('Ctrl+Shift+Space');
    expect(store.getSnapshot().recording).toBeNull();
    expect(draftOf(store)).toEqual({ ...toDraft(SETTINGS), readAloudShortcut: 'Ctrl+Shift+Space' });
    expect(gateway.calls).toEqual(['suspend', 'resume']);
  });

  it('ignores a chord when not recording', async () => {
    const { gateway, store } = await loaded();
    store.record('F1');
    expect(draftOf(store)).toEqual(toDraft(SETTINGS));
    expect(gateway.calls).toEqual([]);
  });

  it('resumes the shortcuts when recording stops without a chord', async () => {
    const { gateway, store } = await loaded();
    store.startRecording('dictateShortcut');
    store.stopRecording();
    store.stopRecording();
    expect(store.getSnapshot().recording).toBeNull();
    expect(draftOf(store)).toEqual(toDraft(SETTINGS));
    expect(gateway.calls).toEqual(['suspend', 'resume']);
  });

  it('shows a failure to suspend or resume', async () => {
    const gateway = new ScriptedGateway();
    gateway.shortcutFailure = new Error('No hotkeys.');
    const { store } = await loaded(gateway);
    store.startRecording('dictateShortcut');
    await flush();
    expect(store.getSnapshot().notice).toEqual({ kind: 'failed', message: 'No hotkeys.' });
    store.edit('voiceHost', 'h');
    store.stopRecording();
    await flush();
    expect(store.getSnapshot().notice).toEqual({ kind: 'failed', message: 'No hotkeys.' });
  });
});
