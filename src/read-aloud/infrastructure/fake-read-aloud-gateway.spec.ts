import { describe, expect, it } from 'vitest';
import type { ReadingStatus, Selection } from '../domain/read-aloud-state.ts';
import { FakeReadAloudGateway, NOTHING_SELECTED } from './fake-read-aloud-gateway.ts';

function selectionOf(text: string | null): Selection[] {
  const gateway = new FakeReadAloudGateway(text);
  const seen: Selection[] = [];
  gateway.onSelection((selection) => seen.push(selection));
  return seen;
}

describe('FakeReadAloudGateway', () => {
  it.each([null, '', '   ', ' \n\t '])('reports nothing selected for %j', (text) => {
    expect(selectionOf(text)).toEqual([{ kind: 'failed', message: NOTHING_SELECTED }]);
  });

  it('tells the person what to do when nothing is selected', () => {
    expect(NOTHING_SELECTED).toBe('Select some text first, then press the read-aloud shortcut.');
  });

  it('reports the selection with its length', () => {
    expect(selectionOf('hello')).toEqual([{ kind: 'ready', preview: 'hello', charCount: 5 }]);
  });

  it('trims and collapses whitespace', () => {
    expect(selectionOf('  one \n\n two\t\tthree  ')).toEqual([
      { kind: 'ready', preview: 'one two three', charCount: 13 },
    ]);
  });

  it('emits reading and stopped statuses', async () => {
    const gateway = new FakeReadAloudGateway('hi');
    const seen: ReadingStatus[] = [];
    gateway.onStatus((status) => seen.push(status));
    await gateway.read('whisper');
    await gateway.stop();
    expect(seen).toEqual([{ kind: 'reading', mood: 'whisper' }, { kind: 'stopped' }]);
  });

  it('goes back to waiting when dismissed', async () => {
    const gateway = new FakeReadAloudGateway('hi');
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    await gateway.dismiss();
    expect(seen.at(-1)).toEqual({ kind: 'waiting' });
  });

  it('does nothing without listeners', async () => {
    const gateway = new FakeReadAloudGateway('hi');
    await expect(gateway.read('calm')).resolves.toBeUndefined();
    await expect(gateway.stop()).resolves.toBeUndefined();
    await expect(gateway.dismiss()).resolves.toBeUndefined();
  });
});
