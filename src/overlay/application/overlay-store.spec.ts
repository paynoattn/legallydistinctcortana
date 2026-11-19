import { describe, expect, it, vi } from 'vitest';
import type { OverlayUpdate } from '../domain/overlay-state.ts';
import { OverlayStore } from './overlay-store.ts';
import type { MotionPreference, OverlayGateway, OverlayListeners } from './ports.ts';

function rejected<T>(reason: unknown): Promise<T> {
  const { promise, reject } = Promise.withResolvers<T>();
  reject(reason);
  return promise;
}

class ScriptedGateway implements OverlayGateway {
  readonly calls: string[] = [];
  failWith: unknown = undefined;
  #listeners: OverlayListeners | undefined;

  listen(listeners: OverlayListeners): void {
    this.#listeners = listeners;
  }

  send(update: OverlayUpdate): void {
    this.#listeners?.state(update);
  }

  level(level: number): void {
    this.#listeners?.level(level);
  }

  fix(): Promise<void> {
    this.calls.push('fix');
    return this.#outcome();
  }

  dismiss(): Promise<void> {
    this.calls.push('dismiss');
    return this.#outcome();
  }

  #outcome(): Promise<void> {
    return this.failWith === undefined ? Promise.resolve() : rejected(this.failWith);
  }
}

class ScriptedMotion implements MotionPreference {
  reduced = false;
  #listener: (() => void) | undefined;

  prefersReduced(): boolean {
    return this.reduced;
  }

  onChange(listener: () => void): void {
    this.#listener = listener;
  }

  change(reduced: boolean): void {
    this.reduced = reduced;
    this.#listener?.();
  }
}

function start(reduced = false): {
  gateway: ScriptedGateway;
  motion: ScriptedMotion;
  store: OverlayStore;
} {
  const gateway = new ScriptedGateway();
  const motion = new ScriptedMotion();
  motion.reduced = reduced;
  return { gateway, motion, store: new OverlayStore(gateway, motion) };
}

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

describe('OverlayStore', () => {
  it('starts hidden and silent', () => {
    const { store } = start();
    expect(store.getSnapshot()).toEqual({
      state: { kind: 'hidden' },
      level: 0,
      reduceMotion: false,
    });
  });

  it('starts with reduced motion when the system prefers it', () => {
    expect(start(true).store.getSnapshot().reduceMotion).toBe(true);
  });

  it('shows the state the core sends', () => {
    const { gateway, store } = start();
    gateway.send({ state: { kind: 'thinking' }, reduceMotion: false });
    expect(store.getSnapshot().state).toEqual({ kind: 'thinking' });
  });

  it('follows the level, clamped to 0..1', () => {
    const { gateway, store } = start();
    gateway.send({ state: { kind: 'listening' }, reduceMotion: false });
    gateway.level(0.4);
    expect(store.getSnapshot()).toEqual({
      state: { kind: 'listening' },
      level: 0.4,
      reduceMotion: false,
    });
    gateway.level(3);
    expect(store.getSnapshot().level).toBe(1);
  });

  it('forgets the level when the state changes', () => {
    const { gateway, store } = start();
    gateway.level(0.8);
    gateway.send({ state: { kind: 'thinking' }, reduceMotion: false });
    expect(store.getSnapshot().level).toBe(0);
  });

  it('reduces motion when the core asks, even if the system does not', () => {
    const { gateway, store } = start();
    gateway.send({ state: { kind: 'armed' }, reduceMotion: true });
    expect(store.getSnapshot().reduceMotion).toBe(true);
    gateway.send({ state: { kind: 'armed' }, reduceMotion: false });
    expect(store.getSnapshot().reduceMotion).toBe(false);
  });

  it('reduces motion when the system asks, even if the core does not', () => {
    const { gateway, store } = start(true);
    gateway.send({ state: { kind: 'armed' }, reduceMotion: false });
    expect(store.getSnapshot().reduceMotion).toBe(true);
  });

  it('follows changes to the system preference', () => {
    const { gateway, motion, store } = start();
    gateway.send({ state: { kind: 'listening' }, reduceMotion: false });
    gateway.level(0.5);
    motion.change(true);
    expect(store.getSnapshot()).toEqual({
      state: { kind: 'listening' },
      level: 0.5,
      reduceMotion: true,
    });
  });

  it('stops reducing motion when the system stops, before the core says anything', () => {
    const { motion, store } = start(true);
    motion.change(false);
    expect(store.getSnapshot().reduceMotion).toBe(false);
  });

  it('keeps reducing motion while the core asks for it after the system stops', () => {
    const { gateway, motion, store } = start(true);
    gateway.send({ state: { kind: 'armed' }, reduceMotion: true });
    motion.change(false);
    expect(store.getSnapshot().reduceMotion).toBe(true);
  });

  it('notifies subscribers until they unsubscribe', () => {
    const { gateway, store } = start();
    const listener = vi.fn<() => void>();
    const unsubscribe = store.subscribe(listener);
    gateway.level(0.2);
    unsubscribe();
    gateway.level(0.3);
    expect(listener).toHaveBeenCalledOnce();
  });

  it('asks the core to fix and to dismiss', async () => {
    const { gateway, store } = start();
    store.fix();
    store.dismiss();
    await flush();
    expect(gateway.calls).toEqual(['fix', 'dismiss']);
    expect(store.getSnapshot().state).toEqual({ kind: 'hidden' });
  });

  it('shows a failed command as an error without a fix', async () => {
    const { gateway, store } = start();
    gateway.send({ state: { kind: 'listening' }, reduceMotion: true });
    gateway.level(0.6);
    gateway.failWith = new Error('The core is gone.');
    store.fix();
    await flush();
    expect(store.getSnapshot()).toEqual({
      state: { kind: 'error', reason: 'The core is gone.', fix: null },
      level: 0,
      reduceMotion: true,
    });
  });

  it('shows a failure that is not an Error as text', async () => {
    const { gateway, store } = start();
    gateway.failWith = 'denied';
    store.dismiss();
    await flush();
    expect(store.getSnapshot().state).toEqual({ kind: 'error', reason: 'denied', fix: null });
  });
});
