import { describe, expect, it } from 'vitest';
import { INVOKED_EVENT, WindowEventIpc } from './window-event-ipc.ts';

describe('WindowEventIpc', () => {
  it('announces each invoked command and resolves with null', async () => {
    const target = new EventTarget();
    const announced: unknown[] = [];
    target.addEventListener(INVOKED_EVENT, (event) => {
      announced.push(event instanceof CustomEvent ? event.detail : 'not a custom event');
    });
    await expect(new WindowEventIpc(target).invoke('overlay_fix')).resolves.toBeNull();
    expect(INVOKED_EVENT).toBe('fake-ipc:invoked');
    expect(announced).toEqual(['overlay_fix']);
  });

  it('delivers prefixed custom events as payloads', async () => {
    const target = new EventTarget();
    const payloads: unknown[] = [];
    const subscribed = new WindowEventIpc(target).listen('overlay:level', ({ payload }) => {
      payloads.push(payload);
    });
    await expect(subscribed).resolves.toBeNull();
    target.dispatchEvent(new CustomEvent('fake-ipc:overlay:level', { detail: { level: 1 } }));
    target.dispatchEvent(new Event('fake-ipc:overlay:level'));
    target.dispatchEvent(new CustomEvent('overlay:level', { detail: { level: 0 } }));
    expect(payloads).toEqual([{ level: 1 }, undefined]);
  });
});
