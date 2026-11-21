type Args = Readonly<Record<string, unknown>> | undefined;

export interface Invocation {
  readonly command: string;
  readonly args: Args;
}

interface Bus {
  readonly log: Invocation[];
  readonly handlers: Map<string, (args: Args) => unknown>;
  emit(event: string, payload: unknown): void;
}

interface CoreControls {
  capture(selection: unknown): void;
  sentence(index: number, startsInMs: number, speechMs: number): void;
  finish(): void;
  fail(message: string): void;
  voicesFailWith(message: string | null): void;
}

export interface CoreSetup {
  readonly selection: unknown;
  readonly voices: readonly unknown[];
  readonly voicesFailure: string | null;
}

interface CoreState {
  selection: unknown;
  reading: number;
  active: boolean;
  voicesFailure: string | null;
}

declare global {
  interface Window {
    qaBus?: Bus;
    qaState?: CoreState;
    qaCore?: CoreControls;
  }
}

export function installTauriBridge(): void {
  const callbacks = new Map<number, (message: unknown) => void>();
  const listeners = new Map<string, number[]>();
  const handlers = new Map<string, (args: Args) => unknown>();
  let nextId = 1;
  const bus: Bus = {
    log: [],
    handlers,
    emit(event, payload) {
      for (const id of listeners.get(event) ?? []) {
        callbacks.get(id)?.({ event, id, payload });
      }
    },
  };
  handlers.set('plugin:event|listen', (args) => {
    const handler = Number(args?.['handler']);
    const event = String(args?.['event']);
    listeners.set(event, [...(listeners.get(event) ?? []), handler]);
    return handler;
  });
  const invoke = (command: string, args: Args): Promise<unknown> => {
    bus.log.push({ command, args });
    return Promise.resolve().then(() => {
      const handler = handlers.get(command);
      return handler === undefined
        ? Promise.reject(new Error(`no command ${command}`))
        : handler(args);
    });
  };
  const transformCallback = (callback: (message: unknown) => void): number => {
    nextId += 1;
    callbacks.set(nextId, callback);
    return nextId;
  };
  Reflect.set(window, '__TAURI_INTERNALS__', { invoke, transformCallback });
  Reflect.set(window, '__TAURI_EVENT_PLUGIN_INTERNALS__', { unregisterListener: () => undefined });
  Reflect.set(window, 'isTauri', true);
  window.qaBus = bus;
}

export function installCommands(setup: CoreSetup): void {
  const bus = window.qaBus;
  if (bus === undefined) {
    throw new Error('the Tauri bridge is not installed');
  }
  const state: CoreState = {
    selection: setup.selection,
    reading: 0,
    active: false,
    voicesFailure: setup.voicesFailure,
  };
  window.qaState = state;
  bus.handlers.set('current_selection', () => state.selection);
  bus.handlers.set('list_voices', () =>
    state.voicesFailure === null ? setup.voices : Promise.reject(new Error(state.voicesFailure)),
  );
  bus.handlers.set('read_aloud', (args) => {
    const request: unknown = args?.['request'];
    const mood: unknown =
      typeof request === 'object' && request !== null ? Reflect.get(request, 'mood') : null;
    state.reading += 1;
    state.active = true;
    bus.emit('read-aloud:status', { kind: 'reading', mood });
    return null;
  });
  bus.handlers.set('stop_reading', () => {
    if (state.active) {
      state.active = false;
      bus.emit('read-aloud:status', { kind: 'stopped' });
    }
    return null;
  });
  bus.handlers.set('dismiss_read_aloud', () => null);
}

export function installControls(): void {
  const bus = window.qaBus;
  const state = window.qaState;
  if (bus === undefined || state === undefined) {
    throw new Error('the fake core is not installed');
  }
  const end = (status: unknown): void => {
    if (state.active) {
      state.active = false;
      bus.emit('read-aloud:status', status);
    }
  };
  window.qaCore = {
    capture(next) {
      end({ kind: 'stopped' });
      state.selection = next;
      bus.emit('read-aloud:selection', next);
    },
    sentence(index, startsInMs, speechMs) {
      bus.emit('read-aloud:sentence', { reading: state.reading, index, startsInMs, speechMs });
    },
    finish() {
      end({ kind: 'finished' });
    },
    fail(message) {
      end({ kind: 'failed', message });
    },
    voicesFailWith(message) {
      state.voicesFailure = message;
    },
  };
}
