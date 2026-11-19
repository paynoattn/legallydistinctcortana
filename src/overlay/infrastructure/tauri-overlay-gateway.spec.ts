import { describe, expect, it } from 'vitest';
import type { OverlayUpdate } from '../domain/overlay-state.ts';
import {
  DISMISS_COMMAND,
  FIX_COMMAND,
  LEVEL_EVENT,
  parseLevel,
  parseUpdate,
  STATE_EVENT,
  TauriOverlayGateway,
  UNEXPECTED_MESSAGE,
} from './tauri-overlay-gateway.ts';

function rejected<T>(reason: unknown): Promise<T> {
  const { promise, reject } = Promise.withResolvers<T>();
  reject(reason);
  return promise;
}

type Handler = (event: { readonly payload: unknown }) => void;

class FakeTauri {
  readonly invocations: string[] = [];
  readonly handlers = new Map<string, Handler>();
  listenFailure: unknown = undefined;
  invokeFailure: unknown = undefined;

  readonly invoke = (command: string): Promise<unknown> => {
    this.invocations.push(command);
    return this.invokeFailure === undefined ? Promise.resolve(null) : rejected(this.invokeFailure);
  };

  readonly listen = (event: string, handler: Handler): Promise<unknown> => {
    if (this.listenFailure !== undefined) {
      return rejected(this.listenFailure);
    }
    this.handlers.set(event, handler);
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

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

function start(): {
  tauri: FakeTauri;
  updates: OverlayUpdate[];
  levels: number[];
} {
  const tauri = new FakeTauri();
  const updates: OverlayUpdate[] = [];
  const levels: number[] = [];
  new TauriOverlayGateway(tauri.invoke, tauri.listen).listen({
    state: (update) => updates.push(update),
    level: (level) => levels.push(level),
  });
  return { tauri, updates, levels };
}

const UNEXPECTED: OverlayUpdate = {
  state: { kind: 'error', reason: UNEXPECTED_MESSAGE, fix: null },
  reduceMotion: false,
};

describe('parseUpdate', () => {
  it.each(['hidden', 'armed', 'listening', 'thinking', 'inserted', 'speaking'])(
    'accepts the %s state',
    (kind) => {
      expect(parseUpdate({ state: { kind }, reduceMotion: true })).toEqual({
        state: { kind },
        reduceMotion: true,
      });
    },
  );

  it.each(['sound_settings', 'settings', null])('accepts an error with the fix %s', (fix) => {
    expect(
      parseUpdate({ state: { kind: 'error', reason: 'No mic.', fix }, reduceMotion: false }),
    ).toEqual({ state: { kind: 'error', reason: 'No mic.', fix }, reduceMotion: false });
  });

  it.each([
    ['nothing', undefined],
    ['null', null],
    ['a string', 'listening'],
    ['no state', { reduceMotion: false }],
    ['an unknown kind', { state: { kind: 'dancing' }, reduceMotion: false }],
    ['no reduceMotion', { state: { kind: 'armed' } }],
    ['a text reduceMotion', { state: { kind: 'armed' }, reduceMotion: 'yes' }],
    ['an error without a reason', { state: { kind: 'error', fix: null }, reduceMotion: false }],
    [
      'an error with a numeric reason',
      { state: { kind: 'error', reason: 1, fix: null }, reduceMotion: false },
    ],
    [
      'an error without a fix',
      { state: { kind: 'error', reason: 'No mic.' }, reduceMotion: false },
    ],
    [
      'an error with an unknown fix',
      { state: { kind: 'error', reason: 'No mic.', fix: 'reboot' }, reduceMotion: false },
    ],
  ])('rejects %s', (_label, payload) => {
    expect(parseUpdate(payload)).toBeNull();
  });
});

describe('parseLevel', () => {
  it('accepts a number', () => {
    expect(parseLevel({ level: 0.25 })).toBe(0.25);
  });

  it.each([
    ['nothing', undefined],
    ['a bare number', 0.5],
    ['text', { level: '0.5' }],
    ['not a number', { level: Number.NaN }],
    ['infinity', { level: Number.POSITIVE_INFINITY }],
  ])('rejects %s', (_label, payload) => {
    expect(parseLevel(payload)).toBeNull();
  });
});

describe('TauriOverlayGateway events', () => {
  it('passes state updates through', async () => {
    const { tauri, updates } = start();
    await flush();
    tauri.emit(STATE_EVENT, { state: { kind: 'armed' }, reduceMotion: false });
    expect(updates).toEqual([{ state: { kind: 'armed' }, reduceMotion: false }]);
  });

  it('turns an unexpected state into an error', async () => {
    const { tauri, updates } = start();
    await flush();
    tauri.emit(STATE_EVENT, { state: { kind: 'dancing' } });
    expect(updates).toEqual([UNEXPECTED]);
  });

  it('passes levels through', async () => {
    const { tauri, levels, updates } = start();
    await flush();
    tauri.emit(LEVEL_EVENT, { level: 0.7 });
    expect(levels).toEqual([0.7]);
    expect(updates).toEqual([]);
  });

  it('turns an unexpected level into an error', async () => {
    const { tauri, levels, updates } = start();
    await flush();
    tauri.emit(LEVEL_EVENT, { level: 'loud' });
    expect(levels).toEqual([]);
    expect(updates).toEqual([UNEXPECTED]);
  });

  it('shows a failed subscription as an error', async () => {
    const tauri = new FakeTauri();
    tauri.listenFailure = new Error('No event bridge.');
    const updates: OverlayUpdate[] = [];
    new TauriOverlayGateway(tauri.invoke, tauri.listen).listen({
      state: (update) => updates.push(update),
      level: () => undefined,
    });
    await flush();
    const failure: OverlayUpdate = {
      state: { kind: 'error', reason: 'No event bridge.', fix: null },
      reduceMotion: false,
    };
    expect(updates).toEqual([failure, failure]);
  });

  it('shows a failure that is not an Error as text', async () => {
    const tauri = new FakeTauri();
    tauri.listenFailure = 'denied';
    const updates: OverlayUpdate[] = [];
    new TauriOverlayGateway(tauri.invoke, tauri.listen).listen({
      state: (update) => updates.push(update),
      level: () => undefined,
    });
    await flush();
    expect(updates[0]?.state).toEqual({ kind: 'error', reason: 'denied', fix: null });
  });
});

describe('TauriOverlayGateway commands', () => {
  it('explains unexpected updates in words', () => {
    expect(UNEXPECTED_MESSAGE).toBe('The app sent an update this window does not understand.');
  });

  it('invokes the fix and dismiss commands', async () => {
    const tauri = new FakeTauri();
    const gateway = new TauriOverlayGateway(tauri.invoke, tauri.listen);
    await gateway.fix();
    await gateway.dismiss();
    expect(tauri.invocations).toEqual([FIX_COMMAND, DISMISS_COMMAND]);
    expect([FIX_COMMAND, DISMISS_COMMAND]).toEqual(['overlay_fix', 'overlay_dismiss']);
  });

  it('listens to the overlay events', () => {
    const { tauri } = start();
    expect([...tauri.handlers.keys()]).toEqual(['overlay:state', 'overlay:level']);
  });

  it('passes command failures on', async () => {
    const tauri = new FakeTauri();
    tauri.invokeFailure = new Error('No window.');
    const gateway = new TauriOverlayGateway(tauri.invoke, tauri.listen);
    await expect(gateway.fix()).rejects.toThrow('No window.');
    await expect(gateway.dismiss()).rejects.toThrow('No window.');
  });
});
