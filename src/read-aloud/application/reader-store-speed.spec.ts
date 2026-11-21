import { describe, expect, it } from 'vitest';
import { at, rig, selected } from './reader-store.fixture.ts';

describe('ReaderStore speed', () => {
  it('reads at the remembered speed', () => {
    const { gateway } = rig({ speed: 1500 });
    gateway.emitSelection({ kind: 'ready', sentences: [{ text: 'Hi.', startsParagraph: true }] });
    expect(gateway.speeds).toEqual([1500]);
  });

  it('restarts the current sentence at the next speed, closes popovers, and remembers it', () => {
    const started = selected();
    at(started, 1);
    started.store.togglePopover('mood');
    started.store.cycleSpeed();
    expect(started.gateway.reads.at(-1)).toEqual({ mood: 'neutral', voice: null, from: 1 });
    expect(started.gateway.speeds.at(-1)).toBe(1250);
    expect(started.preferences.speed).toBe(1250);
    expect(started.store.getSnapshot()).toMatchObject({ speed: 1250, popover: null });
  });

  it('remembers a speed chosen with nothing to read', () => {
    const { gateway, preferences, store } = rig();
    store.cycleSpeed();
    expect(preferences.speed).toBe(1250);
    expect(gateway.reads).toEqual([]);
  });
});
