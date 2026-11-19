import {
  arrayOf,
  integerIn,
  isBoolean,
  isString,
  objectOf,
  oneOf,
  type Check,
} from '../../shared/checks.ts';
import type { SaveResult, SettingsGateway } from '../application/ports.ts';
import type { Connection } from '../domain/connection.ts';
import { SETTINGS_FIELDS, type Settings, type SettingsField } from '../domain/settings.ts';

export type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;
export type Listen = (
  event: string,
  handler: (event: { readonly payload: unknown }) => void,
) => Promise<unknown>;

export const CONNECTION_EVENT = 'connection:status';
export const GET_COMMAND = 'settings_get';
export const DEFAULT_COMMAND = 'settings_default';
export const SAVE_COMMAND = 'settings_save';
export const STATUS_COMMAND = 'connection_status';
export const CHECK_COMMAND = 'check_connection';
export const SUSPEND_COMMAND = 'shortcuts_suspend';
export const RESUME_COMMAND = 'shortcuts_resume';
export const UNEXPECTED_MESSAGE = 'The app sent an update this window does not understand.';

const MAX_PORT = 65_535;
const isPort = integerIn(1, MAX_PORT);

const isSettings: Check<Settings> = objectOf({
  dictateShortcut: isString,
  readAloudShortcut: isString,
  voiceHost: isString,
  sttPort: isPort,
  ttsPort: isPort,
  sttLanguage: isString,
  reduceMotion: isBoolean,
  showOverlay: isBoolean,
  startAtSignIn: isBoolean,
});

const isConnection: Check<Connection> = objectOf({
  kind: oneOf('checking', 'connected', 'partial', 'unreachable'),
  summary: isString,
});

function isSettingsField(value: unknown): value is SettingsField {
  return SETTINGS_FIELDS.some((field) => field === value);
}

const isRejection = objectOf({
  errors: arrayOf(objectOf({ field: isString, message: isString })),
});

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function parsed<T>(check: Check<T>, payload: unknown): T {
  if (!check(payload)) {
    throw new Error(UNEXPECTED_MESSAGE);
  }
  return payload;
}

function rejection(error: unknown): SaveResult {
  if (!isRejection(error)) {
    throw new Error(messageOf(error));
  }
  const { errors } = error;
  const general = errors.filter(({ field }) => !isSettingsField(field));
  if (general.length > 0) {
    throw new Error(general.map(({ message }) => message).join(' '));
  }
  return {
    kind: 'rejected',
    errors: Object.fromEntries(errors.map(({ field, message }) => [field, message])),
  };
}

export class TauriSettingsGateway implements SettingsGateway {
  readonly #invoke: Invoke;
  readonly #listen: Listen;

  constructor(invoke: Invoke, listen: Listen) {
    this.#invoke = invoke;
    this.#listen = listen;
  }

  async load(): Promise<Settings> {
    return parsed(isSettings, await this.#invoke(GET_COMMAND));
  }

  async defaults(): Promise<Settings> {
    return parsed(isSettings, await this.#invoke(DEFAULT_COMMAND));
  }

  save(settings: Settings): Promise<SaveResult> {
    return this.#invoke(SAVE_COMMAND, { settings }).then(
      (payload): SaveResult => ({ kind: 'saved', settings: parsed(isSettings, payload) }),
      rejection,
    );
  }

  async connectionStatus(): Promise<Connection> {
    return parsed(isConnection, await this.#invoke(STATUS_COMMAND));
  }

  async checkConnection(): Promise<Connection> {
    return parsed(isConnection, await this.#invoke(CHECK_COMMAND));
  }

  onConnection(listener: (connection: Connection) => void): void {
    this.#listen(CONNECTION_EVENT, ({ payload }) => {
      listener(isConnection(payload) ? payload : { kind: 'failed', summary: UNEXPECTED_MESSAGE });
    }).catch((error: unknown) => {
      listener({ kind: 'failed', summary: messageOf(error) });
    });
  }

  async suspendShortcuts(): Promise<void> {
    await this.#invoke(SUSPEND_COMMAND);
  }

  async resumeShortcuts(): Promise<void> {
    await this.#invoke(RESUME_COMMAND);
  }
}
