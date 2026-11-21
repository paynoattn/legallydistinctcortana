import { describe, expect, it } from 'vitest';
import { at, flush, rig, selected } from './reader-store.fixture.ts';

describe('ReaderStore mood and voice', () => {
  it('restarts the current sentence in a new mood and remembers it', () => {
    const started = selected();
    at(started, 1);
    started.store.togglePopover('mood');
    started.store.chooseMood('urgent');
    expect(started.gateway.reads.at(-1)).toEqual({ mood: 'urgent', voice: null, from: 1 });
    expect(started.preferences.mood).toBe('urgent');
    expect(started.store.getSnapshot()).toMatchObject({ mood: 'urgent', popover: null });
  });

  it('starts reading in the new mood from the paused sentence', () => {
    const started = selected();
    at(started, 1);
    started.store.togglePlay();
    started.store.chooseMood('cheerful');
    expect(started.gateway.reads.at(-1)).toEqual({ mood: 'cheerful', voice: null, from: 1 });
  });

  it('reads from the first sentence in a new mood after finishing', () => {
    const started = selected();
    started.gateway.emitStatus({ kind: 'finished' });
    started.store.chooseMood('calm');
    expect(started.gateway.reads.at(-1)?.from).toBe(0);
  });

  it('keeps reading when the mood chosen is the current one', () => {
    const started = selected();
    at(started, 1);
    started.store.togglePopover('mood');
    const reads = started.gateway.reads.length;
    started.store.chooseMood('neutral');
    expect(started.gateway.reads).toHaveLength(reads);
    expect(started.store.getSnapshot()).toMatchObject({ mood: 'neutral', popover: null });
  });

  it('remembers a mood chosen with nothing to read', () => {
    const { gateway, preferences, store } = rig();
    store.chooseMood('whisper');
    expect(preferences.mood).toBe('whisper');
    expect(gateway.reads).toEqual([]);
  });

  it('restarts the current sentence in a newly picked voice', () => {
    const started = selected();
    at(started, 2);
    started.voice = { name: 'en_US-amy-medium', speaker: null };
    started.store.voiceChanged();
    expect(started.gateway.reads.at(-1)).toEqual({
      mood: 'neutral',
      voice: { name: 'en_US-amy-medium', speaker: null },
      from: 2,
    });
  });
});

describe('ReaderStore text size, following and popovers', () => {
  it('sizes the text within its limits and remembers the size', () => {
    const { preferences, store } = rig();
    store.larger();
    expect(store.getSnapshot().textSize).toBe(28);
    expect(preferences.textSize).toBe(28);
    store.smaller();
    store.smaller();
    store.smaller();
    expect(store.getSnapshot().textSize).toBe(20);
  });

  it('stops following once and follows again on request', () => {
    const { store } = rig();
    let notified = 0;
    store.subscribe(() => {
      notified += 1;
    });
    store.stopFollowing();
    store.stopFollowing();
    expect(store.getSnapshot().following).toBe(false);
    expect(notified).toBe(1);
    store.follow();
    expect(store.getSnapshot().following).toBe(true);
  });

  it('opens one popover at a time and closes it again', () => {
    const { store } = rig();
    store.togglePopover('mood');
    expect(store.getSnapshot().popover).toBe('mood');
    store.togglePopover('voice');
    expect(store.getSnapshot().popover).toBe('voice');
    store.togglePopover('voice');
    expect(store.getSnapshot().popover).toBeNull();
    store.togglePopover('mood');
    store.closePopover();
    expect(store.getSnapshot().popover).toBeNull();
  });

  it('stops listening when unsubscribed', () => {
    const { store } = rig();
    let notified = 0;
    const unsubscribe = store.subscribe(() => {
      notified += 1;
    });
    unsubscribe();
    store.larger();
    expect(notified).toBe(0);
  });
});

describe('ReaderStore closing', () => {
  it('stops the reading, then dismisses the window', async () => {
    const started = selected();
    started.store.close();
    expect(started.store.getSnapshot().status).toEqual({ kind: 'paused' });
    await flush();
    expect(started.gateway.calls).toEqual(['read', 'stop', 'dismiss']);
  });

  it('stops waiting for a reading to start once closed', () => {
    const started = selected();
    started.gateway.announceReading = false;
    started.store.readSentence(1);
    started.store.close();
    started.gateway.emitStatus({ kind: 'failed', message: 'no speaker' });
    expect(started.store.getSnapshot().status).toEqual({ kind: 'failed', message: 'no speaker' });
  });

  it('keeps a finished status when closing', () => {
    const started = selected();
    started.gateway.emitStatus({ kind: 'finished' });
    started.store.close();
    expect(started.store.getSnapshot().status).toEqual({ kind: 'finished' });
  });

  it('closes an open popover with Escape before closing the window', async () => {
    const started = selected();
    started.store.togglePopover('mood');
    started.store.escape();
    expect(started.store.getSnapshot().popover).toBeNull();
    started.store.escape();
    await flush();
    expect(started.gateway.calls.slice(-2)).toEqual(['stop', 'dismiss']);
  });

  it('fails with the message when closing is refused', async () => {
    const started = selected();
    started.gateway.failWith = new Error('cannot hide');
    started.store.close();
    await flush();
    expect(started.store.getSnapshot().status).toEqual({ kind: 'failed', message: 'cannot hide' });
    expect(started.gateway.calls.at(-1)).toBe('stop');
  });
});
