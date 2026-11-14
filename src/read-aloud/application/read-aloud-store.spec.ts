import { describe, expect, it } from 'vitest';
import type { MoodId } from '../domain/mood.ts';
import { IDLE, type ReadingStatus, type Selection, WAITING } from '../domain/read-aloud-state.ts';
import type { ReadAloudGateway } from './ports.ts';
import { ReadAloudStore } from './read-aloud-store.ts';

class RecordingGateway implements ReadAloudGateway {
  readonly calls: string[] = [];
  failWith: unknown = undefined;
  emitSelection: (selection: Selection) => void = () => undefined;
  emitStatus: (status: ReadingStatus) => void = () => undefined;

  onSelection(listener: (selection: Selection) => void): void {
    this.emitSelection = listener;
  }

  onStatus(listener: (status: ReadingStatus) => void): void {
    this.emitStatus = listener;
  }

  read(mood: MoodId): Promise<void> {
    this.calls.push(`read ${mood}`);
    return this.outcome();
  }

  stop(): Promise<void> {
    this.calls.push('stop');
    return this.outcome();
  }

  dismiss(): Promise<void> {
    this.calls.push('dismiss');
    return this.outcome();
  }

  outcome(): Promise<void> {
    const { promise, reject, resolve } = Promise.withResolvers<undefined>();
    if (this.failWith === undefined) {
      resolve(undefined);
    } else {
      reject(this.failWith);
    }
    return promise;
  }
}

const READY: Selection = { kind: 'ready', preview: 'hello', charCount: 5 };

function start(): { gateway: RecordingGateway; store: ReadAloudStore } {
  const gateway = new RecordingGateway();
  return { gateway, store: new ReadAloudStore(gateway) };
}

const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

describe('ReadAloudStore', () => {
  it('starts waiting, idle, in round 0', () => {
    const { store } = start();
    expect(store.getSnapshot()).toEqual({ selection: WAITING, status: IDLE, round: 0 });
  });

  it('takes a new selection, resets the status and starts a new round', () => {
    const { gateway, store } = start();
    gateway.emitStatus({ kind: 'stopped' });
    gateway.emitSelection(READY);
    expect(store.getSnapshot()).toEqual({ selection: READY, status: IDLE, round: 1 });
    gateway.emitSelection(WAITING);
    expect(store.getSnapshot()).toEqual({ selection: WAITING, status: IDLE, round: 2 });
  });

  it('takes status updates without starting a new round', () => {
    const { gateway, store } = start();
    gateway.emitSelection(READY);
    gateway.emitStatus({ kind: 'finished' });
    expect(store.getSnapshot()).toEqual({
      selection: READY,
      status: { kind: 'finished' },
      round: 1,
    });
  });

  it('shows reading straight away when a mood is chosen, then asks the gateway', () => {
    const { gateway, store } = start();
    gateway.emitSelection(READY);
    store.choose('calm');
    expect(store.getSnapshot()).toEqual({
      selection: READY,
      status: { kind: 'reading', mood: 'calm' },
      round: 1,
    });
    expect(gateway.calls).toEqual(['read calm']);
  });

  it('forwards stop and dismiss without changing the snapshot', async () => {
    const { gateway, store } = start();
    const before = store.getSnapshot();
    store.stop();
    store.dismiss();
    await flush();
    expect(gateway.calls).toEqual(['stop', 'dismiss']);
    expect(store.getSnapshot()).toBe(before);
  });

  it.each([
    [
      'choose',
      (store: ReadAloudStore): void => {
        store.choose('calm');
      },
    ],
    [
      'stop',
      (store: ReadAloudStore): void => {
        store.stop();
      },
    ],
    [
      'dismiss',
      (store: ReadAloudStore): void => {
        store.dismiss();
      },
    ],
  ])('fails with the error message when %s is rejected', async (_name, act) => {
    const { gateway, store } = start();
    gateway.failWith = new Error('no audio device');
    act(store);
    await flush();
    expect(store.getSnapshot().status).toEqual({ kind: 'failed', message: 'no audio device' });
  });

  it('fails with a plain string rejection as is', async () => {
    const { gateway, store } = start();
    gateway.failWith = 'speaker is busy';
    store.stop();
    await flush();
    expect(store.getSnapshot().status).toEqual({ kind: 'failed', message: 'speaker is busy' });
  });

  it('notifies subscribers on every change until they unsubscribe', () => {
    const { gateway, store } = start();
    let notified = 0;
    const unsubscribe = store.subscribe(() => {
      notified += 1;
    });
    gateway.emitSelection(READY);
    gateway.emitStatus({ kind: 'stopped' });
    store.choose('mad');
    expect(notified).toBe(3);
    unsubscribe();
    gateway.emitStatus({ kind: 'finished' });
    expect(notified).toBe(3);
  });

  it('keeps the snapshot identity stable between changes', () => {
    const { gateway, store } = start();
    gateway.emitSelection(READY);
    expect(store.getSnapshot()).toBe(store.getSnapshot());
  });
});
