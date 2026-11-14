import { describe, expect, it } from 'vitest';
import type { ReadingStatus, Selection } from '../domain/read-aloud-state.ts';
import {
  CURRENT_SELECTION_COMMAND,
  SELECTION_EVENT,
  STATUS_EVENT,
  TauriReadAloudGateway,
  UNEXPECTED_MESSAGE,
} from './tauri-read-aloud-gateway.ts';

type Handler = (event: { readonly payload: unknown }) => void;

function settle<T>(value: T, failure: unknown): Promise<T> {
  const { promise, reject, resolve } = Promise.withResolvers<T>();
  if (failure === undefined) {
    resolve(value);
  } else {
    reject(failure);
  }
  return promise;
}

class FakeTauri {
  readonly invocations: { command: string; args: Record<string, unknown> | undefined }[] = [];
  readonly handlers = new Map<string, Handler>();
  invokeFailure: unknown = undefined;
  listenFailure: unknown = undefined;
  currentSelection: unknown = null;
  currentSelectionFailure: unknown = undefined;
  currentGate: PromiseWithResolvers<unknown> | undefined = undefined;
  listenGate: PromiseWithResolvers<undefined> | undefined = undefined;

  readonly invoke = (command: string, args?: Record<string, unknown>): Promise<unknown> => {
    this.invocations.push({ command, args });
    if (command === CURRENT_SELECTION_COMMAND) {
      if (this.currentGate !== undefined) {
        return this.currentGate.promise;
      }
      return settle(this.currentSelection, this.currentSelectionFailure);
    }
    return settle(null, this.invokeFailure);
  };

  readonly listen = (event: string, handler: Handler): Promise<unknown> => {
    if (this.listenFailure !== undefined) {
      return settle(null, this.listenFailure);
    }
    this.handlers.set(event, handler);
    if (this.listenGate !== undefined) {
      return this.listenGate.promise;
    }
    return Promise.resolve(() => undefined);
  };

  emit(event: string, payload: unknown): void {
    const handler = this.handlers.get(event);
    if (handler === undefined) {
      throw new Error(`nobody listens to ${event}`);
    }
    handler({ payload });
  }
}

function start(): { tauri: FakeTauri; gateway: TauriReadAloudGateway } {
  const tauri = new FakeTauri();
  return { tauri, gateway: new TauriReadAloudGateway(tauri.invoke, tauri.listen) };
}

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

describe('TauriReadAloudGateway commands', () => {
  it('invokes read_aloud with the mood and nulls when no voice is chosen', async () => {
    const { tauri, gateway } = start();
    await gateway.read('urgent', null);
    expect(tauri.invocations).toEqual([
      {
        command: 'read_aloud',
        args: { request: { mood: 'urgent', voice: null, speaker: null } },
      },
    ]);
  });

  it('invokes read_aloud with the chosen voice and speaker', async () => {
    const { tauri, gateway } = start();
    await gateway.read('calm', { name: 'en_US-libritts-high', speaker: 'p239' });
    expect(tauri.invocations).toEqual([
      {
        command: 'read_aloud',
        args: { request: { mood: 'calm', voice: 'en_US-libritts-high', speaker: 'p239' } },
      },
    ]);
  });

  it('invokes read_aloud with a voice that has no speaker', async () => {
    const { tauri, gateway } = start();
    await gateway.read('calm', { name: 'en_US-amy-medium', speaker: null });
    expect(tauri.invocations).toEqual([
      {
        command: 'read_aloud',
        args: { request: { mood: 'calm', voice: 'en_US-amy-medium', speaker: null } },
      },
    ]);
  });

  it('invokes stop_reading and dismiss_read_aloud without arguments', async () => {
    const { tauri, gateway } = start();
    await gateway.stop();
    await gateway.dismiss();
    expect(tauri.invocations).toEqual([
      { command: 'stop_reading', args: undefined },
      { command: 'dismiss_read_aloud', args: undefined },
    ]);
  });

  it('rejects read with the host error, even a plain string', async () => {
    const { tauri, gateway } = start();
    tauri.invokeFailure = 'no speaker';
    await expect(gateway.read('calm', null)).rejects.toBe('no speaker');
  });

  it('rejects stop with the host error', async () => {
    const { tauri, gateway } = start();
    tauri.invokeFailure = 'no speaker';
    await expect(gateway.stop()).rejects.toBe('no speaker');
  });

  it('rejects dismiss with the host error', async () => {
    const { tauri, gateway } = start();
    tauri.invokeFailure = 'no speaker';
    await expect(gateway.dismiss()).rejects.toBe('no speaker');
  });
});

describe('TauriReadAloudGateway current selection', () => {
  it('asks the host for the selection it is already holding', async () => {
    const { tauri, gateway } = start();
    gateway.onSelection(() => undefined);
    await flush();
    expect(CURRENT_SELECTION_COMMAND).toBe('current_selection');
    expect(tauri.invocations).toEqual([{ command: 'current_selection', args: undefined }]);
  });

  it.each<Selection>([
    { kind: 'ready', preview: 'held', charCount: 4 },
    { kind: 'failed', message: 'nothing' },
  ])('delivers a valid held selection %j', async (held) => {
    const { tauri, gateway } = start();
    tauri.currentSelection = held;
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    await flush();
    expect(seen).toEqual([held]);
  });

  it.each<unknown>([
    null,
    undefined,
    'ready',
    { kind: 'ready', preview: 'x' },
    { kind: 'waiting' },
  ])('ignores the held selection %j when it is empty or invalid', async (held) => {
    const { tauri, gateway } = start();
    tauri.currentSelection = held;
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    await flush();
    expect(seen).toEqual([]);
  });

  it.each([
    ['an Error', new Error('host is busy'), 'host is busy'],
    ['a plain string', 'host is gone', 'host is gone'],
  ])('fails with the message when the host rejects with %s', async (_name, failure, text) => {
    const { tauri, gateway } = start();
    tauri.currentSelectionFailure = failure;
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    await flush();
    expect(seen).toEqual([{ kind: 'failed', message: text }]);
  });

  it('waits for the listener to be registered before asking for the held selection', async () => {
    const { tauri, gateway } = start();
    tauri.listenGate = Promise.withResolvers();
    gateway.onSelection(() => undefined);
    await flush();
    expect(tauri.invocations).toEqual([]);
    tauri.listenGate.resolve(undefined);
    await flush();
    expect(tauri.invocations).toEqual([{ command: 'current_selection', args: undefined }]);
  });

  it('ignores the held selection once a live selection event has arrived', async () => {
    const { tauri, gateway } = start();
    tauri.currentGate = Promise.withResolvers();
    const live: Selection = { kind: 'ready', preview: 'live', charCount: 4 };
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    await flush();
    tauri.emit(SELECTION_EVENT, live);
    tauri.currentGate.resolve({ kind: 'ready', preview: 'stale', charCount: 5 });
    await flush();
    expect(seen).toEqual([live]);
  });

  it('delivers the held selection when no live event came first', async () => {
    const { tauri, gateway } = start();
    tauri.currentGate = Promise.withResolvers();
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    await flush();
    tauri.currentGate.resolve({ kind: 'ready', preview: 'held', charCount: 4 });
    await flush();
    expect(seen).toEqual([{ kind: 'ready', preview: 'held', charCount: 4 }]);
  });

  it('never asks for the held selection when listening fails', async () => {
    const { tauri, gateway } = start();
    tauri.listenFailure = 'denied';
    gateway.onSelection(() => undefined);
    await flush();
    expect(tauri.invocations).toEqual([]);
  });

  it('does not ask the host for the selection when listening for status', () => {
    const { tauri, gateway } = start();
    gateway.onStatus(() => undefined);
    expect(tauri.invocations).toEqual([]);
  });
});

describe('TauriReadAloudGateway events', () => {
  it('listens to the selection and status events', () => {
    const { tauri, gateway } = start();
    gateway.onSelection(() => undefined);
    gateway.onStatus(() => undefined);
    expect([...tauri.handlers.keys()].sort((a, b) => a.localeCompare(b))).toEqual(
      [SELECTION_EVENT, STATUS_EVENT].sort((a, b) => a.localeCompare(b)),
    );
    expect(SELECTION_EVENT).toBe('read-aloud:selection');
    expect(STATUS_EVENT).toBe('read-aloud:status');
  });

  it.each<Selection>([
    { kind: 'ready', preview: 'hello', charCount: 5 },
    { kind: 'ready', preview: '', charCount: 0 },
    { kind: 'failed', message: 'nothing' },
  ])('passes a valid selection %j through', (payload) => {
    const { tauri, gateway } = start();
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    tauri.emit(SELECTION_EVENT, payload);
    expect(seen).toEqual([payload]);
  });

  it.each<ReadingStatus>([
    { kind: 'reading', mood: 'mad' },
    { kind: 'finished' },
    { kind: 'stopped' },
    { kind: 'failed', message: 'boom' },
  ])('passes a valid status %j through', (payload) => {
    const { tauri, gateway } = start();
    const seen: ReadingStatus[] = [];
    gateway.onStatus((status) => seen.push(status));
    tauri.emit(STATUS_EVENT, payload);
    expect(seen).toEqual([payload]);
  });

  it.each<unknown>([
    null,
    'ready',
    {},
    { kind: 'waiting' },
    { kind: 'ready', preview: 'x' },
    { kind: 'ready', preview: 'x', charCount: -1 },
    { kind: 'ready', preview: 'x', charCount: 1.5 },
    { kind: 'ready', preview: 3, charCount: 1 },
    { kind: 'failed' },
  ])('turns the invalid selection %j into a failure', (payload) => {
    const { tauri, gateway } = start();
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    tauri.emit(SELECTION_EVENT, payload);
    expect(seen).toEqual([{ kind: 'failed', message: UNEXPECTED_MESSAGE }]);
  });

  it.each<unknown>([
    undefined,
    { kind: 'idle' },
    { kind: 'reading' },
    { kind: 'reading', mood: 'sad' },
    { kind: 'reading', mood: 3 },
    { kind: 'failed' },
    { kind: 'failed', message: 4 },
  ])('turns the invalid status %j into a failure', (payload) => {
    const { tauri, gateway } = start();
    const seen: ReadingStatus[] = [];
    gateway.onStatus((status) => seen.push(status));
    tauri.emit(STATUS_EVENT, payload);
    expect(seen).toEqual([{ kind: 'failed', message: UNEXPECTED_MESSAGE }]);
  });

  it('words the unexpected-payload failure for people', () => {
    expect(UNEXPECTED_MESSAGE).toBe('The app sent an update this window does not understand.');
  });

  it('reports a failed subscription with the error message', async () => {
    const { tauri, gateway } = start();
    tauri.listenFailure = new Error('events unavailable');
    const selections: Selection[] = [];
    const statuses: ReadingStatus[] = [];
    gateway.onSelection((selection) => selections.push(selection));
    gateway.onStatus((status) => statuses.push(status));
    await flush();
    expect(selections).toEqual([{ kind: 'failed', message: 'events unavailable' }]);
    expect(statuses).toEqual([{ kind: 'failed', message: 'events unavailable' }]);
  });

  it('reports a plain string subscription failure as is', async () => {
    const { tauri, gateway } = start();
    tauri.listenFailure = 'denied';
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    await flush();
    expect(seen).toEqual([{ kind: 'failed', message: 'denied' }]);
  });
});
