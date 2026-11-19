import type { Connection } from '../domain/connection.ts';
import {
  type Draft,
  type FieldErrors,
  parseDraft,
  type Settings,
  type ShortcutField,
  toDraft,
} from '../domain/settings.ts';
import type { SaveResult, SettingsGateway } from './ports.ts';

type Listener = () => void;

type Form =
  | { readonly kind: 'loading' }
  | { readonly kind: 'failed'; readonly message: string }
  | { readonly kind: 'ready'; readonly saved: Draft; readonly draft: Draft };

export type Notice =
  | { readonly kind: 'none' }
  | { readonly kind: 'saved' }
  | { readonly kind: 'failed'; readonly message: string };

export interface SettingsSnapshot {
  readonly form: Form;
  readonly coreErrors: FieldErrors;
  readonly notice: Notice;
  readonly saving: boolean;
  readonly recording: ShortcutField | null;
  readonly connection: Connection | null;
  readonly checking: boolean;
}

const NO_NOTICE: Notice = { kind: 'none' };

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function withoutField(errors: FieldErrors, field: keyof Draft): FieldErrors {
  return Object.fromEntries(Object.entries(errors).filter(([key]) => key !== field));
}

function readyWith(settings: Settings): Form {
  const saved = toDraft(settings);
  return { kind: 'ready', saved, draft: saved };
}

export class SettingsStore {
  readonly #listeners = new Set<Listener>();
  readonly #gateway: SettingsGateway;
  #snapshot: SettingsSnapshot = {
    form: { kind: 'loading' },
    coreErrors: {},
    notice: NO_NOTICE,
    saving: false,
    recording: null,
    connection: null,
    checking: false,
  };

  constructor(gateway: SettingsGateway) {
    this.#gateway = gateway;
    gateway.onConnection((connection) => {
      this.#commit({ connection });
    });
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  };

  readonly getSnapshot = (): SettingsSnapshot => this.#snapshot;

  readonly load = (): void => {
    this.#gateway.load().then(this.#showSaved, (error: unknown) => {
      this.#commit({ form: { kind: 'failed', message: messageOf(error) } });
    });
    this.#gateway.connectionStatus().then((connection) => {
      if (this.#snapshot.connection === null) {
        this.#commit({ connection });
      }
    }, this.#connectionFailed);
  };

  readonly edit = <K extends keyof Draft>(field: K, value: Draft[K]): void => {
    const { form, coreErrors } = this.#snapshot;
    if (form.kind === 'ready') {
      this.#commit({
        form: { ...form, draft: { ...form.draft, [field]: value } },
        coreErrors: withoutField(coreErrors, field),
        notice: NO_NOTICE,
      });
    }
  };

  readonly revert = (): void => {
    const { form } = this.#snapshot;
    if (form.kind === 'ready') {
      this.#commit({ form: { ...form, draft: form.saved }, coreErrors: {}, notice: NO_NOTICE });
    }
  };

  readonly save = (): void => {
    const { form, saving } = this.#snapshot;
    if (form.kind !== 'ready' || saving) {
      return;
    }
    const parsed = parseDraft(form.draft);
    if (parsed.kind === 'valid') {
      this.#commit({ saving: true, notice: NO_NOTICE });
      this.#gateway.save(parsed.settings).then(this.#saved, this.#failed);
    }
  };

  readonly resetShortcut = (field: ShortcutField): void => {
    this.#gateway.defaults().then((defaults) => {
      this.edit(field, defaults[field]);
    }, this.#failed);
  };

  readonly startRecording = (field: ShortcutField): void => {
    const wasRecording = this.#snapshot.recording !== null;
    this.#commit({ recording: field });
    if (!wasRecording) {
      this.#gateway.suspendShortcuts().catch(this.#failed);
    }
  };

  readonly record = (chord: string): void => {
    const field = this.#snapshot.recording;
    if (field !== null) {
      this.stopRecording();
      this.edit(field, chord);
    }
  };

  readonly stopRecording = (): void => {
    if (this.#snapshot.recording !== null) {
      this.#commit({ recording: null });
      this.#gateway.resumeShortcuts().catch(this.#failed);
    }
  };

  readonly checkConnection = (): void => {
    this.#commit({ checking: true });
    this.#gateway.checkConnection().then(this.#checked, this.#connectionFailed);
  };

  readonly #showSaved = (settings: Settings): void => {
    this.#commit({ form: readyWith(settings) });
  };

  readonly #saved = (result: SaveResult): void => {
    if (result.kind === 'saved') {
      this.#commit({
        form: readyWith(result.settings),
        coreErrors: {},
        notice: { kind: 'saved' },
        saving: false,
      });
    } else {
      this.#commit({ coreErrors: result.errors, saving: false });
    }
  };

  readonly #failed = (error: unknown): void => {
    this.#commit({ notice: { kind: 'failed', message: messageOf(error) }, saving: false });
  };

  readonly #checked = (connection: Connection): void => {
    this.#commit({ connection, checking: false });
  };

  readonly #connectionFailed = (error: unknown): void => {
    this.#checked({ kind: 'failed', summary: messageOf(error) });
  };

  #commit(change: Partial<SettingsSnapshot>): void {
    this.#snapshot = { ...this.#snapshot, ...change };
    for (const listener of this.#listeners) {
      listener();
    }
  }
}
