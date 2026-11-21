import { describe, expect, it } from 'vitest';
import { at, flush, rig, selected, THREE } from './reader-store.fixture.ts';

describe('ReaderStore play and pause', () => {
  it('pauses at the current sentence and resumes from its start', () => {
    const started = selected();
    at(started, 1);
    started.store.togglePlay();
    expect(started.gateway.calls.at(-1)).toBe('stop');
    expect(started.store.getSnapshot()).toMatchObject({ status: { kind: 'paused' }, sentence: 1 });
    started.store.togglePlay();
    expect(started.gateway.reads.at(-1)?.from).toBe(1);
    expect(started.store.getSnapshot().status).toEqual({ kind: 'reading' });
  });

  it('treats a stop from the app while reading as a pause', () => {
    const started = selected();
    at(started, 1);
    started.gateway.emitStatus({ kind: 'stopped' });
    expect(started.store.getSnapshot()).toMatchObject({ status: { kind: 'paused' }, sentence: 1 });
  });

  it('finishes with every sentence read and nothing lit', () => {
    const started = selected();
    at(started, 2);
    started.gateway.emitStatus({ kind: 'finished' });
    expect(started.store.getSnapshot()).toMatchObject({
      status: { kind: 'finished' },
      sentence: 3,
      word: null,
    });
  });

  it('reads again from the first sentence after finishing', () => {
    const started = selected();
    at(started, 2);
    started.gateway.emitStatus({ kind: 'finished' });
    started.store.togglePlay();
    expect(started.gateway.reads.map((read) => read.from)).toEqual([0, 0]);
  });

  it('ignores an end that comes while paused', () => {
    const started = selected();
    started.store.togglePlay();
    started.gateway.emitStatus({ kind: 'finished' });
    expect(started.store.getSnapshot().status).toEqual({ kind: 'paused' });
  });

  it('ignores the end of the previous reading while a new one starts', () => {
    const started = selected();
    started.gateway.announceReading = false;
    started.store.readSentence(1);
    started.gateway.emitStatus({ kind: 'stopped' });
    started.gateway.emitStatus({ kind: 'failed', message: 'old reading' });
    expect(started.store.getSnapshot().status).toEqual({ kind: 'reading' });
  });

  it('shows a failure of the voice with nothing lit, and reads again from that sentence', () => {
    const started = selected();
    at(started, 1);
    started.gateway.emitStatus({ kind: 'failed', message: 'no voice' });
    expect(started.store.getSnapshot()).toMatchObject({
      status: { kind: 'failed', message: 'no voice' },
      sentence: 1,
      word: null,
    });
    started.store.togglePlay();
    expect(started.gateway.reads.at(-1)?.from).toBe(1);
  });

  it.each([
    ['an Error', new Error('no speaker'), 'no speaker'],
    ['a plain string', 'no speaker', 'no speaker'],
  ])('fails with the message when reading is refused with %s', async (_name, failure, text) => {
    const started = rig();
    started.gateway.failWith = failure;
    started.gateway.emitSelection({ kind: 'ready', sentences: THREE });
    await flush();
    expect(started.store.getSnapshot().status).toEqual({ kind: 'failed', message: text });
  });

  it('fails when pausing is refused', async () => {
    const started = selected();
    started.gateway.failWith = 'busy';
    started.store.togglePlay();
    await flush();
    expect(started.store.getSnapshot().status).toEqual({ kind: 'failed', message: 'busy' });
  });
});

describe('ReaderStore moving by sentence', () => {
  it('reads the next sentence, but not past the last', () => {
    const started = selected();
    at(started, 1);
    started.store.next();
    expect(started.gateway.reads.at(-1)?.from).toBe(2);
    at(started, 2);
    started.store.next();
    expect(started.gateway.reads).toHaveLength(2);
  });

  it.each([
    [0, 0],
    [1500, 0],
    [1501, 1],
  ])('goes back after %i ms of a sentence to sentence %i', (played, expected) => {
    const started = selected();
    at(started, 1);
    started.clock.advance(played);
    started.store.previous();
    expect(started.gateway.reads.at(-1)?.from).toBe(expected);
  });

  it('measures how long a sentence played from when it started, not from the epoch', () => {
    const started = selected();
    started.clock.advance(5000);
    at(started, 1);
    started.clock.advance(1000);
    started.store.previous();
    expect(started.gateway.reads.at(-1)?.from).toBe(0);
  });

  it('restarts the first sentence instead of going before it', () => {
    const started = selected();
    at(started, 0);
    started.store.previous();
    expect(started.gateway.reads.at(-1)?.from).toBe(0);
  });

  it('goes back one while paused, however long the sentence played', () => {
    const started = selected();
    at(started, 2);
    started.clock.advance(5000);
    started.store.togglePlay();
    started.store.previous();
    expect(started.gateway.reads.at(-1)?.from).toBe(1);
  });

  it('goes back to the last sentence after finishing', () => {
    const started = selected();
    started.gateway.emitStatus({ kind: 'finished' });
    started.store.previous();
    expect(started.gateway.reads.at(-1)?.from).toBe(2);
  });

  it('reads from a clicked sentence, keeping it inside the text', () => {
    const started = selected();
    started.store.readSentence(2);
    started.store.readSentence(7);
    started.store.readSentence(-3);
    expect(started.gateway.reads.map((read) => read.from)).toEqual([0, 2, 2, 0]);
  });

  it('follows the voice again when reading from another sentence', () => {
    const started = selected();
    started.store.stopFollowing();
    started.store.readSentence(2);
    expect(started.store.getSnapshot().following).toBe(true);
  });
});
