import { act, render, screen } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { OverlayStore } from '../application/overlay-store.ts';
import type { MotionPreference, OverlayGateway, OverlayListeners } from '../application/ports.ts';
import type { OverlayState } from '../domain/overlay-state.ts';
import { OverlayView } from './overlay-view.tsx';

class ScriptedGateway implements OverlayGateway {
  readonly calls: string[] = [];
  #listeners: OverlayListeners | undefined;

  listen(listeners: OverlayListeners): void {
    this.#listeners = listeners;
  }

  send(state: OverlayState, reduceMotion = false): void {
    act(() => {
      this.#listeners?.state({ state, reduceMotion });
    });
  }

  level(level: number): void {
    act(() => {
      this.#listeners?.level(level);
    });
  }

  fix(): Promise<void> {
    this.calls.push('fix');
    return Promise.resolve();
  }

  dismiss(): Promise<void> {
    this.calls.push('dismiss');
    return Promise.resolve();
  }
}

const STILL: MotionPreference = { prefersReduced: () => false, onChange: () => undefined };

function show(): ScriptedGateway {
  const gateway = new ScriptedGateway();
  render(<OverlayView store={new OverlayStore(gateway, STILL)} />);
  return gateway;
}

describe('OverlayView', () => {
  it('starts hidden, out of the accessibility tree', () => {
    show();
    const mark = screen.getByTestId('mark');
    expect(mark.dataset['state']).toBe('hidden');
    expect(mark.getAttribute('aria-hidden')).toBe('true');
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('shows each state the core sends', () => {
    const gateway = show();
    gateway.send({ kind: 'thinking' });
    expect(screen.getByTestId('mark').dataset['state']).toBe('thinking');
    expect(screen.getByTestId('core').style.opacity).toBe('0.7');
    expect(screen.getByTestId('halo').style.opacity).toBe('0.18');
  });

  it('scales the core and the halo with the level while listening', () => {
    const gateway = show();
    gateway.send({ kind: 'listening' });
    gateway.level(1);
    expect(screen.getByTestId('core').style.transform).toBe('scale(1.35)');
    expect(screen.getByTestId('halo').style.opacity).toBe('1');
  });

  it('leaves the core untransformed outside listening', () => {
    const gateway = show();
    gateway.send({ kind: 'armed' });
    expect(screen.getByTestId('core').style.transform).toBe('');
  });

  it('drops the motion class unless motion is reduced', () => {
    const gateway = show();
    gateway.send({ kind: 'listening' });
    expect(screen.getByTestId('overlay').className).toBe('overlay');
    gateway.send({ kind: 'listening' }, true);
    gateway.level(1);
    expect(screen.getByTestId('overlay').className).toBe('overlay reduce-motion');
    expect(screen.getByTestId('core').style.transform).toBe('');
  });

  it('shows rings only while speaking', () => {
    const gateway = show();
    expect(screen.queryByTestId('ring')).toBeNull();
    gateway.send({ kind: 'speaking' });
    expect(screen.getByTestId('ring').className).toBe('lp-ring');
  });

  it('shows the error reason and its fix, and clicking each tells the core', async () => {
    const gateway = show();
    gateway.send({ kind: 'error', reason: 'Microphone not found.', fix: 'sound_settings' });
    await userEvent.click(screen.getByRole('button', { name: 'Choose a mic' }));
    await userEvent.click(screen.getByRole('button', { name: 'Microphone not found.' }));
    expect(gateway.calls).toEqual(['fix', 'dismiss']);
  });

  it('offers no fix when the core has none', () => {
    const gateway = show();
    gateway.send({ kind: 'error', reason: 'Transcription failed.', fix: null });
    expect(screen.getAllByRole('button').map((button) => button.textContent)).toEqual([
      'Transcription failed.',
    ]);
  });
});
