import { describe, expect, it } from 'vitest';
import type { Selection } from '../domain/read-aloud-state.ts';
import { CURRENT_SELECTION_COMMAND, SELECTION_EVENT } from './read-aloud-ipc-names.ts';
import { flush, start } from './tauri-read-aloud-gateway.fixture.ts';

describe('TauriReadAloudGateway commands', () => {
  it('invokes read_aloud with the mood, the first sentence, and nulls when no voice is chosen', async () => {
    const { tauri, gateway } = start();
    await gateway.read({ mood: 'urgent', voice: null, fromSentence: 0, speed: 1000 });
    expect(tauri.invocations).toEqual([
      {
        command: 'read_aloud',
        args: {
          request: { mood: 'urgent', voice: null, speaker: null, fromSentence: 0, speed: 1000 },
        },
      },
    ]);
  });

  it('invokes read_aloud from a later sentence at the chosen speed', async () => {
    const { tauri, gateway } = start();
    await gateway.read({ mood: 'calm', voice: null, fromSentence: 7, speed: 1500 });
    expect(tauri.invocations).toEqual([
      {
        command: 'read_aloud',
        args: {
          request: { mood: 'calm', voice: null, speaker: null, fromSentence: 7, speed: 1500 },
        },
      },
    ]);
  });

  it('invokes read_aloud with the chosen voice and speaker', async () => {
    const { tauri, gateway } = start();
    await gateway.read({
      mood: 'calm',
      voice: { name: 'en_US-libritts-high', speaker: 'p239' },
      fromSentence: 0,
      speed: 1000,
    });
    expect(tauri.invocations).toEqual([
      {
        command: 'read_aloud',
        args: {
          request: {
            mood: 'calm',
            voice: 'en_US-libritts-high',
            speaker: 'p239',
            fromSentence: 0,
            speed: 1000,
          },
        },
      },
    ]);
  });

  it('invokes read_aloud with a voice that has no speaker', async () => {
    const { tauri, gateway } = start();
    await gateway.read({
      mood: 'calm',
      voice: { name: 'en_US-amy-medium', speaker: null },
      fromSentence: 0,
      speed: 1000,
    });
    expect(tauri.invocations).toEqual([
      {
        command: 'read_aloud',
        args: {
          request: {
            mood: 'calm',
            voice: 'en_US-amy-medium',
            speaker: null,
            fromSentence: 0,
            speed: 1000,
          },
        },
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
    await expect(
      gateway.read({ mood: 'calm', voice: null, fromSentence: 0, speed: 1000 }),
    ).rejects.toBe('no speaker');
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
    { kind: 'ready', sentences: [{ text: 'held', startsParagraph: true }] },
    { kind: 'failed', message: 'nothing' },
  ])('delivers a valid held selection %j', async (held) => {
    const { tauri, gateway } = start();
    tauri.currentSelection = held;
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    await flush();
    expect(seen).toEqual([held]);
  });

  it.each<unknown>([null, undefined, 'ready', { kind: 'ready' }, { kind: 'waiting' }])(
    'ignores the held selection %j when it is empty or invalid',
    async (held) => {
      const { tauri, gateway } = start();
      tauri.currentSelection = held;
      const seen: Selection[] = [];
      gateway.onSelection((selection) => seen.push(selection));
      await flush();
      expect(seen).toEqual([]);
    },
  );

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
    const live: Selection = { kind: 'ready', sentences: [{ text: 'live', startsParagraph: true }] };
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    await flush();
    tauri.emit(SELECTION_EVENT, live);
    tauri.currentGate.resolve({
      kind: 'ready',
      sentences: [{ text: 'stale', startsParagraph: true }],
    });
    await flush();
    expect(seen).toEqual([live]);
  });

  it('delivers the held selection when no live event came first', async () => {
    const { tauri, gateway } = start();
    tauri.currentGate = Promise.withResolvers();
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    await flush();
    tauri.currentGate.resolve({
      kind: 'ready',
      sentences: [{ text: 'held', startsParagraph: true }],
    });
    await flush();
    expect(seen).toEqual([{ kind: 'ready', sentences: [{ text: 'held', startsParagraph: true }] }]);
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
