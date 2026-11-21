import { describe, expect, it } from 'vitest';
import { at, rig, selected, sentences, GOOD_MORNING } from './reader-store.fixture.ts';

describe('ReaderStore before any selection', () => {
  it('waits in the remembered mood and text size, following the voice', () => {
    const { store } = rig({ mood: 'calm', textSize: 31, speed: 1500 });
    expect(store.getSnapshot()).toEqual({
      selection: { kind: 'waiting' },
      round: 0,
      mood: 'calm',
      textSize: 31,
      speed: 1500,
      status: { kind: 'idle' },
      sentence: 0,
      word: null,
      following: true,
      popover: null,
    });
  });

  it('starts Neutral at 25 px when nothing is remembered', () => {
    const { store } = rig();
    expect(store.getSnapshot()).toMatchObject({ mood: 'neutral', textSize: 25, speed: 1000 });
  });

  it('reads nothing when asked to play without text', () => {
    const { gateway, store } = rig();
    store.togglePlay();
    store.next();
    store.previous();
    store.readSentence(0);
    expect(gateway.reads).toEqual([]);
  });
});

describe('ReaderStore when a selection arrives', () => {
  it('reads it at once from the first sentence in the mood and voice in use', () => {
    const started = rig({ mood: 'calm' });
    started.voice = { name: 'en_US-amy-medium', speaker: null };
    started.gateway.emitSelection({ kind: 'ready', sentences: GOOD_MORNING });
    expect(started.gateway.reads).toEqual([
      { mood: 'calm', voice: { name: 'en_US-amy-medium', speaker: null }, from: 0 },
    ]);
    expect(started.store.getSnapshot()).toMatchObject({
      selection: {
        kind: 'ready',
        sentences: [
          { text: 'Good morning.', startsParagraph: true },
          { text: 'Two emails need you.', startsParagraph: false },
        ],
      },
      round: 1,
      status: { kind: 'reading' },
      sentence: 0,
      word: null,
    });
  });

  it('reads nothing when the selection failed, and follows the voice again', () => {
    const { gateway, store } = rig();
    store.stopFollowing();
    gateway.emitSelection({ kind: 'failed', message: 'nothing' });
    expect(gateway.reads).toEqual([]);
    expect(store.getSnapshot()).toMatchObject({
      round: 1,
      status: { kind: 'idle' },
      following: true,
    });
  });

  it('shows a failure the app reports before any selection', () => {
    const { gateway, store } = rig();
    gateway.emitStatus({ kind: 'failed', message: 'no speaker' });
    expect(store.getSnapshot().status).toEqual({ kind: 'failed', message: 'no speaker' });
  });

  it('stops waiting for the old reading to start once a selection fails', () => {
    const started = rig();
    started.gateway.announceReading = false;
    started.gateway.emitSelection({ kind: 'ready', sentences: GOOD_MORNING });
    started.gateway.emitSelection({ kind: 'failed', message: 'nothing' });
    started.gateway.emitStatus({ kind: 'failed', message: 'no speaker' });
    expect(started.store.getSnapshot().status).toEqual({ kind: 'failed', message: 'no speaker' });
  });

  it('starts a new round from the first sentence, following, with menus closed', () => {
    const started = selected();
    at(started, 2);
    started.store.stopFollowing();
    started.store.togglePopover('voice');
    started.gateway.emitSelection({ kind: 'ready', sentences: sentences('Back at noon.') });
    expect(started.store.getSnapshot()).toMatchObject({
      round: 2,
      sentence: 0,
      word: null,
      following: true,
      popover: null,
    });
    expect(started.gateway.reads.at(-1)?.from).toBe(0);
  });
});

describe('ReaderStore word timing', () => {
  it('lights the first word of the latest sentence the voice has started', () => {
    const started = selected();
    at(started, 0);
    expect(started.store.getSnapshot()).toMatchObject({ sentence: 0, word: 0 });
    at(started, 1);
    expect(started.store.getSnapshot()).toMatchObject({ sentence: 1, word: 0 });
  });

  it('forgets the timing of the previous reading when reading from another sentence', () => {
    const started = selected();
    at(started, 1);
    started.store.readSentence(2);
    started.clock.advance(16);
    expect(started.store.getSnapshot()).toMatchObject({ sentence: 2, word: null });
  });

  it('lights the word being spoken from the sentence report and the clock', () => {
    const started = selected(GOOD_MORNING);
    at(started, 1, 1200);
    expect(started.store.getSnapshot()).toMatchObject({ sentence: 1, word: 0 });
    started.clock.advance(700);
    expect(started.store.getSnapshot()).toMatchObject({ sentence: 1, word: 2 });
  });

  it('waits for a sentence that starts later', () => {
    const started = selected(GOOD_MORNING);
    started.gateway.emitSentence({ index: 0, startsInMs: 500, speechMs: 800 });
    started.clock.advance(400);
    expect(started.store.getSnapshot()).toMatchObject({ sentence: 0, word: null });
    started.clock.advance(100);
    expect(started.store.getSnapshot()).toMatchObject({ sentence: 0, word: 0 });
  });

  it('keeps asking for frames while reading and stops when paused', () => {
    const started = selected();
    expect(started.clock.frames).toHaveLength(1);
    started.clock.advance(16);
    expect(started.clock.frames).toHaveLength(1);
    started.store.togglePlay();
    started.clock.advance(16);
    expect(started.clock.frames).toHaveLength(0);
  });

  it('asks for one frame at a time', () => {
    const started = selected();
    started.store.readSentence(1);
    started.store.readSentence(2);
    expect(started.clock.frames).toHaveLength(1);
  });

  it('does not notify listeners when a frame changes nothing', () => {
    const started = selected();
    at(started, 0);
    let notified = 0;
    started.store.subscribe(() => {
      notified += 1;
    });
    started.clock.advance(1);
    expect(notified).toBe(0);
  });

  it('ignores reports for a reading the window has not seen start yet', () => {
    const started = selected();
    started.gateway.announceReading = false;
    started.store.readSentence(2);
    at(started, 0);
    expect(started.store.getSnapshot()).toMatchObject({ sentence: 2, word: null });
    started.gateway.emitStatus({ kind: 'reading' });
    at(started, 2);
    expect(started.store.getSnapshot()).toMatchObject({ sentence: 2, word: 0 });
  });

  it('ignores reports while paused', () => {
    const started = selected();
    started.store.togglePlay();
    at(started, 1);
    expect(started.store.getSnapshot()).toMatchObject({ sentence: 0 });
  });

  it('times a report for a sentence it does not know as having no words', () => {
    const started = selected();
    at(started, 9);
    expect(started.store.getSnapshot()).toMatchObject({ sentence: 9, word: null });
  });
});
