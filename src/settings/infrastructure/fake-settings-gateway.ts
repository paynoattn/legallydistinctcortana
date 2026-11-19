import type { SaveResult, SettingsGateway } from '../application/ports.ts';
import type { Connection } from '../domain/connection.ts';
import type { Settings } from '../domain/settings.ts';

export const FAKE_DEFAULTS: Settings = {
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

export const LANGUAGE_MESSAGE = 'Use "auto" or a language code such as "en".';

const LANGUAGE = /^(?:auto|[a-z]{2,3})$/;

function connectedTo(host: string): Connection {
  return { kind: 'connected', summary: `Speech recognition and voice are reachable at ${host}.` };
}

export class FakeSettingsGateway implements SettingsGateway {
  #settings: Settings = FAKE_DEFAULTS;
  #onConnection: ((connection: Connection) => void) | undefined;

  load(): Promise<Settings> {
    return Promise.resolve(this.#settings);
  }

  defaults(): Promise<Settings> {
    return Promise.resolve(FAKE_DEFAULTS);
  }

  save(settings: Settings): Promise<SaveResult> {
    if (!LANGUAGE.test(settings.sttLanguage)) {
      return Promise.resolve({ kind: 'rejected', errors: { sttLanguage: LANGUAGE_MESSAGE } });
    }
    this.#settings = { ...settings, voiceHost: settings.voiceHost.toLowerCase() };
    return Promise.resolve({ kind: 'saved', settings: this.#settings });
  }

  connectionStatus(): Promise<Connection> {
    return Promise.resolve(connectedTo(this.#settings.voiceHost));
  }

  checkConnection(): Promise<Connection> {
    this.#onConnection?.({ kind: 'checking', summary: 'Checking the voice server.' });
    return this.connectionStatus();
  }

  onConnection(listener: (connection: Connection) => void): void {
    this.#onConnection = listener;
  }

  suspendShortcuts(): Promise<void> {
    return Promise.resolve();
  }

  resumeShortcuts(): Promise<void> {
    return Promise.resolve();
  }
}
