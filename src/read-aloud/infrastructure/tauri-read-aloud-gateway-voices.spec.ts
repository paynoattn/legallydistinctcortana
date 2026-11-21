import { describe, expect, it } from 'vitest';
import { LIST_VOICES_COMMAND } from './read-aloud-ipc-names.ts';
import { TauriReadAloudGateway, UNEXPECTED_MESSAGE } from './tauri-read-aloud-gateway.ts';

const AMY = {
  name: 'en_US-amy-medium',
  description: 'amy (medium)',
  languages: ['en_US'],
  speakers: [],
};

function start(reply: unknown): {
  invocations: { command: string; args: Record<string, unknown> | undefined }[];
  gateway: TauriReadAloudGateway;
} {
  const invocations: { command: string; args: Record<string, unknown> | undefined }[] = [];
  const invoke = (command: string, args?: Record<string, unknown>): Promise<unknown> => {
    invocations.push({ command, args });
    const { promise, reject, resolve } = Promise.withResolvers<unknown>();
    if (reply instanceof Error) {
      reject(reply);
    } else {
      resolve(reply);
    }
    return promise;
  };
  const listen = (): Promise<() => void> => Promise.resolve(() => undefined);
  return { invocations, gateway: new TauriReadAloudGateway(invoke, listen) };
}

describe('TauriReadAloudGateway voices', () => {
  it('lists the voices the host reports', async () => {
    const { invocations, gateway } = start([AMY]);
    expect(await gateway.listVoices()).toEqual([AMY]);
    expect(LIST_VOICES_COMMAND).toBe('list_voices');
    expect(invocations).toEqual([{ command: 'list_voices', args: undefined }]);
  });

  it('keeps the speakers of a multi-speaker voice', async () => {
    const libritts = { ...AMY, name: 'libritts', speakers: ['p239', 'p240'] };
    const { gateway } = start([libritts]);
    expect(await gateway.listVoices()).toEqual([libritts]);
  });

  it('accepts an empty list', async () => {
    const { gateway } = start([]);
    expect(await gateway.listVoices()).toEqual([]);
  });

  it.each<unknown>([
    null,
    undefined,
    'voices',
    {},
    [{}],
    [{ ...AMY, name: 3 }],
    [{ ...AMY, description: undefined }],
    [{ ...AMY, languages: 'en_US' }],
    [{ ...AMY, languages: [1] }],
    [{ ...AMY, speakers: undefined }],
    [{ ...AMY, speakers: [1] }],
  ])('rejects the invalid voice list %j', async (payload) => {
    const { gateway } = start(payload);
    await expect(gateway.listVoices()).rejects.toThrow(UNEXPECTED_MESSAGE);
  });

  it('rejects with the host error when listing fails', async () => {
    const failure = new Error('no voice server');
    const { gateway } = start(failure);
    await expect(gateway.listVoices()).rejects.toBe(failure);
  });
});
