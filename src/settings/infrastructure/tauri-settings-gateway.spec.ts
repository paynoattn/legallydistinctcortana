import { describe, expect, it } from 'vitest';
import type { Connection } from '../domain/connection.ts';
import type { Settings } from '../domain/settings.ts';
import {
  CHECK_COMMAND,
  CONNECTION_EVENT,
  DEFAULT_COMMAND,
  GET_COMMAND,
  RESUME_COMMAND,
  SAVE_COMMAND,
  STATUS_COMMAND,
  SUSPEND_COMMAND,
  TauriSettingsGateway,
  UNEXPECTED_MESSAGE,
} from './tauri-settings-gateway.ts';

function rejected<T>(reason: unknown): Promise<T> {
  const { promise, reject } = Promise.withResolvers<T>();
  reject(reason);
  return promise;
}

type Handler = (event: { readonly payload: unknown }) => void;

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

class FakeTauri {
  readonly invocations: { command: string; args: Record<string, unknown> | undefined }[] = [];
  readonly handlers = new Map<string, Handler>();
  readonly answers = new Map<string, unknown>();
  failure: unknown = undefined;
  listenFailure: unknown = undefined;

  readonly invoke = (command: string, args?: Record<string, unknown>): Promise<unknown> => {
    this.invocations.push({ command, args });
    return this.failure === undefined
      ? Promise.resolve(this.answers.get(command) ?? null)
      : rejected(this.failure);
  };

  readonly listen = (event: string, handler: Handler): Promise<unknown> => {
    if (this.listenFailure !== undefined) {
      return rejected(this.listenFailure);
    }
    this.handlers.set(event, handler);
    return Promise.resolve(() => undefined);
  };

  emit(payload: unknown): void {
    this.handlers.get(CONNECTION_EVENT)?.({ payload });
  }
}

function start(): { tauri: FakeTauri; gateway: TauriSettingsGateway } {
  const tauri = new FakeTauri();
  return { tauri, gateway: new TauriSettingsGateway(tauri.invoke, tauri.listen) };
}

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

describe('TauriSettingsGateway names', () => {
  it('explains unexpected answers in words', () => {
    expect(UNEXPECTED_MESSAGE).toBe('The app sent an update this window does not understand.');
  });

  it('uses the agreed command and event names', () => {
    expect([
      GET_COMMAND,
      DEFAULT_COMMAND,
      SAVE_COMMAND,
      STATUS_COMMAND,
      CHECK_COMMAND,
      SUSPEND_COMMAND,
      RESUME_COMMAND,
      CONNECTION_EVENT,
    ]).toEqual([
      'settings_get',
      'settings_default',
      'settings_save',
      'connection_status',
      'check_connection',
      'shortcuts_suspend',
      'shortcuts_resume',
      'connection:status',
    ]);
  });
});

describe('TauriSettingsGateway settings', () => {
  it('loads the settings', async () => {
    const { tauri, gateway } = start();
    tauri.answers.set(GET_COMMAND, SETTINGS);
    await expect(gateway.load()).resolves.toEqual(SETTINGS);
    expect(tauri.invocations).toEqual([{ command: GET_COMMAND, args: undefined }]);
  });

  it('loads the defaults', async () => {
    const { tauri, gateway } = start();
    tauri.answers.set(DEFAULT_COMMAND, { ...SETTINGS, dictateShortcut: 'F2' });
    await expect(gateway.defaults()).resolves.toEqual({ ...SETTINGS, dictateShortcut: 'F2' });
  });

  it.each([
    ['nothing', null],
    ['a missing field', { ...SETTINGS, startAtSignIn: undefined }],
    ['a text port', { ...SETTINGS, sttPort: '10300' }],
    ['a fractional port', { ...SETTINGS, ttsPort: 10.5 }],
    ['port zero', { ...SETTINGS, sttPort: 0 }],
    ['a port above 65535', { ...SETTINGS, ttsPort: 65_536 }],
  ])('refuses settings with %s', async (_label, payload) => {
    const { tauri, gateway } = start();
    tauri.answers.set(GET_COMMAND, payload);
    await expect(gateway.load()).rejects.toThrow(UNEXPECTED_MESSAGE);
  });

  it('accepts the edge ports', async () => {
    const { tauri, gateway } = start();
    tauri.answers.set(GET_COMMAND, { ...SETTINGS, sttPort: 1, ttsPort: 65_535 });
    await expect(gateway.load()).resolves.toEqual({ ...SETTINGS, sttPort: 1, ttsPort: 65_535 });
  });

  it('saves and returns what the core stored', async () => {
    const { tauri, gateway } = start();
    tauri.answers.set(SAVE_COMMAND, { ...SETTINGS, readAloudShortcut: 'Ctrl+Alt+R' });
    const result = await gateway.save({ ...SETTINGS, readAloudShortcut: 'ctrl+alt+r' });
    expect(result).toEqual({ kind: 'saved', settings: SETTINGS });
    expect(tauri.invocations).toEqual([
      {
        command: SAVE_COMMAND,
        args: { settings: { ...SETTINGS, readAloudShortcut: 'ctrl+alt+r' } },
      },
    ]);
  });

  it('refuses an unexpected save answer', async () => {
    const { gateway } = start();
    await expect(gateway.save(SETTINGS)).rejects.toThrow(UNEXPECTED_MESSAGE);
  });

  it('turns the core field errors into a rejection', async () => {
    const { tauri, gateway } = start();
    tauri.failure = {
      errors: [
        { field: 'sttPort', message: 'Port in use.' },
        { field: 'voiceHost', message: 'Unknown host.' },
      ],
    };
    await expect(gateway.save(SETTINGS)).resolves.toEqual({
      kind: 'rejected',
      errors: { sttPort: 'Port in use.', voiceHost: 'Unknown host.' },
    });
  });

  it.each([
    ['an Error', new Error('Disk full.'), 'Disk full.'],
    ['text', 'Disk full.', 'Disk full.'],
    [
      'not about a field',
      {
        errors: [
          { field: 'general', message: "Couldn't save: disk full" },
          { field: 'sttPort', message: 'Port in use.' },
        ],
      },
      "Couldn't save: disk full",
    ],
    [
      'two messages not about a field',
      {
        errors: [
          { field: 'general', message: 'First.' },
          { field: 'general', message: 'Second.' },
        ],
      },
      'First. Second.',
    ],
    ['malformed', { errors: [{ field: 'sttPort' }] }, '[object Object]'],
  ])('passes on a save failure that is %s', async (_label, failure, message) => {
    const { tauri, gateway } = start();
    tauri.failure = failure;
    await expect(gateway.save(SETTINGS)).rejects.toThrow(message);
  });
});

describe('TauriSettingsGateway connection', () => {
  const PARTIAL: Connection = { kind: 'partial', summary: 'Only voice.' };

  it.each([STATUS_COMMAND, CHECK_COMMAND])('reads the connection with %s', async (command) => {
    const { tauri, gateway } = start();
    tauri.answers.set(command, PARTIAL);
    const read =
      command === STATUS_COMMAND ? gateway.connectionStatus() : gateway.checkConnection();
    await expect(read).resolves.toEqual(PARTIAL);
    expect(tauri.invocations).toEqual([{ command, args: undefined }]);
  });

  it.each(['checking', 'connected', 'partial', 'unreachable'])('accepts %s', async (kind) => {
    const { tauri, gateway } = start();
    tauri.answers.set(STATUS_COMMAND, { kind, summary: 'S.' });
    await expect(gateway.connectionStatus()).resolves.toEqual({ kind, summary: 'S.' });
  });

  it('refuses an unknown connection kind', async () => {
    const { tauri, gateway } = start();
    tauri.answers.set(CHECK_COMMAND, { kind: 'failed', summary: 'S.' });
    await expect(gateway.checkConnection()).rejects.toThrow(UNEXPECTED_MESSAGE);
  });

  it('passes on pushed connections and flags unexpected ones', async () => {
    const { tauri, gateway } = start();
    const seen: Connection[] = [];
    gateway.onConnection((connection) => seen.push(connection));
    await flush();
    tauri.emit(PARTIAL);
    tauri.emit({ kind: 'partial' });
    expect(seen).toEqual([PARTIAL, { kind: 'failed', summary: UNEXPECTED_MESSAGE }]);
  });

  it.each([
    [new Error('No bridge.'), 'No bridge.'],
    ['denied', 'denied'],
  ])('shows a failed subscription', async (failure, summary) => {
    const { tauri, gateway } = start();
    tauri.listenFailure = failure;
    const seen: Connection[] = [];
    gateway.onConnection((connection) => seen.push(connection));
    await flush();
    expect(seen).toEqual([{ kind: 'failed', summary }]);
  });
});

describe('TauriSettingsGateway shortcuts', () => {
  it('suspends and resumes the global shortcuts', async () => {
    const { tauri, gateway } = start();
    await gateway.suspendShortcuts();
    await gateway.resumeShortcuts();
    expect(tauri.invocations.map(({ command }) => command)).toEqual([
      SUSPEND_COMMAND,
      RESUME_COMMAND,
    ]);
  });
});
