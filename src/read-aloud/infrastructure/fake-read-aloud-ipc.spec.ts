import { describe, expect, it } from 'vitest';
import {
  CAPTURE_EVENT,
  FAKE_VOICES,
  FakeReadAloudIpc,
  INVOKED_EVENT,
  NOTHING_SELECTED,
  selectionFor,
  sentencesOf,
} from './fake-read-aloud-ipc.ts';

function recorder(target: EventTarget, event: string): unknown[] {
  const seen: unknown[] = [];
  target.addEventListener(event, (fired) => {
    seen.push(fired instanceof CustomEvent ? fired.detail : 'not a custom event');
  });
  return seen;
}

function listening(ipc: FakeReadAloudIpc, event: string): unknown[] {
  const seen: unknown[] = [];
  void ipc.listen(event, ({ payload }) => {
    seen.push(payload);
  });
  return seen;
}

describe('sentencesOf', () => {
  it('splits after sentence punctuation and opens a paragraph at each line', () => {
    expect(sentencesOf('Good morning.\nTwo emails need you. Lunch is at noon!')).toEqual([
      { text: 'Good morning.', startsParagraph: true },
      { text: 'Two emails need you.', startsParagraph: true },
      { text: 'Lunch is at noon!', startsParagraph: false },
    ]);
  });

  it('keeps decimals together and drops blank lines', () => {
    expect(sentencesOf('Pi is 3.14 today…  Really?\n\n  \nYes')).toEqual([
      { text: 'Pi is 3.14 today…', startsParagraph: true },
      { text: 'Really?', startsParagraph: false },
      { text: 'Yes', startsParagraph: true },
    ]);
  });
});

describe('selectionFor', () => {
  it.each([null, '', '   ', ' \n\t '])('reports nothing selected for %j', (text) => {
    expect(selectionFor(text)).toEqual({ kind: 'failed', message: NOTHING_SELECTED });
  });

  it('tells the person what to do when nothing is selected', () => {
    expect(NOTHING_SELECTED).toBe('Select some text first, then press the read-aloud shortcut.');
  });

  it('offers the sentences of the selection', () => {
    expect(selectionFor('Hi. There.')).toEqual({
      kind: 'ready',
      sentences: [
        { text: 'Hi.', startsParagraph: true },
        { text: 'There.', startsParagraph: false },
      ],
    });
  });
});

describe('FakeReadAloudIpc', () => {
  it('answers the held selection and the voices', async () => {
    const ipc = new FakeReadAloudIpc(new EventTarget(), 'Hello.');
    await expect(ipc.invoke('current_selection')).resolves.toEqual(selectionFor('Hello.'));
    await expect(ipc.invoke('list_voices')).resolves.toBe(FAKE_VOICES);
  });

  it('announces every command it is asked to run', async () => {
    const target = new EventTarget();
    const invoked = recorder(target, INVOKED_EVENT);
    const ipc = new FakeReadAloudIpc(target, null);
    await ipc.invoke('dismiss_read_aloud');
    await ipc.invoke('read_aloud', { request: { mood: 'calm' } });
    expect(invoked).toEqual([
      { command: 'dismiss_read_aloud', args: undefined },
      { command: 'read_aloud', args: { request: { mood: 'calm' } } },
    ]);
  });

  it('reports a reading in the requested mood, and neutral when the request is odd', async () => {
    const ipc = new FakeReadAloudIpc(new EventTarget(), null);
    const statuses = listening(ipc, 'read-aloud:status');
    await expect(ipc.invoke('read_aloud', { request: { mood: 'calm' } })).resolves.toBeNull();
    await ipc.invoke('read_aloud', { mood: 'calm' });
    expect(statuses).toEqual([
      { kind: 'reading', mood: 'calm' },
      { kind: 'reading', mood: 'neutral' },
    ]);
  });

  it('reports a stop', async () => {
    const ipc = new FakeReadAloudIpc(new EventTarget(), null);
    const statuses = listening(ipc, 'read-aloud:status');
    await expect(ipc.invoke('stop_reading')).resolves.toBeNull();
    expect(statuses).toEqual([{ kind: 'stopped' }]);
  });

  it('sends a newly captured selection to its listeners', () => {
    const target = new EventTarget();
    const ipc = new FakeReadAloudIpc(target, null);
    const selections = listening(ipc, 'read-aloud:selection');
    target.dispatchEvent(new CustomEvent(CAPTURE_EVENT, { detail: 'Back at noon.' }));
    target.dispatchEvent(new CustomEvent(CAPTURE_EVENT, { detail: 4 }));
    target.dispatchEvent(new Event(CAPTURE_EVENT));
    expect(selections).toEqual([
      selectionFor('Back at noon.'),
      selectionFor(null),
      selectionFor(null),
    ]);
  });

  it('uses the event names the acceptance steps dispatch and listen for', () => {
    expect(CAPTURE_EVENT).toBe('fake-ipc:capture');
    expect(INVOKED_EVENT).toBe('fake-ipc:invoked');
  });

  it('passes nothing on for a plain event', () => {
    const target = new EventTarget();
    const ipc = new FakeReadAloudIpc(target, null);
    const payloads = listening(ipc, 'read-aloud:sentence');
    target.dispatchEvent(new Event('fake-ipc:read-aloud:sentence'));
    expect(payloads).toEqual([undefined]);
  });
});
