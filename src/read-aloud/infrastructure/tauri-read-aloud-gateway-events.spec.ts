import { describe, expect, it } from 'vitest';
import type { ReadingStatus, Selection, SentenceReport } from '../domain/read-aloud-state.ts';
import { SELECTION_EVENT, SENTENCE_EVENT, STATUS_EVENT } from './read-aloud-ipc-names.ts';
import { UNEXPECTED_MESSAGE } from './tauri-read-aloud-gateway.ts';
import { flush, start } from './tauri-read-aloud-gateway.fixture.ts';

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
    {
      kind: 'ready',
      sentences: [
        { text: 'Hello.', startsParagraph: true },
        { text: 'Again.', startsParagraph: false },
      ],
    },
    { kind: 'ready', sentences: [] },
    { kind: 'failed', message: 'nothing' },
  ])('passes a valid selection %j through', (payload) => {
    const { tauri, gateway } = start();
    const seen: Selection[] = [];
    gateway.onSelection((selection) => seen.push(selection));
    tauri.emit(SELECTION_EVENT, payload);
    expect(seen).toEqual([payload]);
  });

  it.each<ReadingStatus>([
    { kind: 'reading' },
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
    { kind: 'ready' },
    { kind: 'ready', sentences: 'x' },
    { kind: 'ready', sentences: [{ text: 'x' }] },
    { kind: 'ready', sentences: [{ text: 3, startsParagraph: true }] },
    { kind: 'ready', sentences: [{ text: 'x', startsParagraph: 'yes' }] },
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
    { kind: 3 },
    { kind: 'failed' },
    { kind: 'failed', message: 4 },
  ])('turns the invalid status %j into a failure', (payload) => {
    const { tauri, gateway } = start();
    const seen: ReadingStatus[] = [];
    gateway.onStatus((status) => seen.push(status));
    tauri.emit(STATUS_EVENT, payload);
    expect(seen).toEqual([{ kind: 'failed', message: UNEXPECTED_MESSAGE }]);
  });

  it('listens to the sentence event and passes valid reports through', () => {
    const { tauri, gateway } = start();
    const seen: SentenceReport[] = [];
    gateway.onSentence((report) => seen.push(report));
    tauri.emit(SENTENCE_EVENT, { reading: 3, index: 2, startsInMs: 1500, speechMs: 900 });
    expect(SENTENCE_EVENT).toBe('read-aloud:sentence');
    expect(seen).toEqual([{ reading: 3, index: 2, startsInMs: 1500, speechMs: 900 }]);
  });

  it.each<unknown>([
    null,
    {},
    { index: -1, startsInMs: 0, speechMs: 0 },
    { index: 0, startsInMs: 1.5, speechMs: 0 },
    { index: 0, startsInMs: 0, speechMs: '9' },
    { index: 0, speechMs: 9 },
  ])('drops the invalid sentence report %j', (payload) => {
    const { tauri, gateway } = start();
    const seen: SentenceReport[] = [];
    gateway.onSentence((report) => seen.push(report));
    tauri.emit(SENTENCE_EVENT, payload);
    expect(seen).toEqual([]);
  });

  it('survives a failed sentence subscription without reporting anything', async () => {
    const { tauri, gateway } = start();
    tauri.listenFailure = 'denied';
    const seen: SentenceReport[] = [];
    gateway.onSentence((report) => seen.push(report));
    await flush();
    expect(seen).toEqual([]);
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
