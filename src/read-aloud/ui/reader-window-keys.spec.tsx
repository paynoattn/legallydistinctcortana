import { act, fireEvent, render, screen } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { FAKE_VOICES } from '../infrastructure/fake-read-aloud-ipc.ts';
import { stores, type Stores } from './reader-window.fixture.ts';
import { ReaderWindow } from './reader-window.tsx';

interface Rig extends Stores {
  readonly user: ReturnType<typeof userEvent.setup>;
  readonly unmount: () => void;
}

function start(): Rig {
  const rig = stores(FAKE_VOICES, act);
  const { unmount } = render(<ReaderWindow store={rig.store} voices={rig.voices} />);
  return { ...rig, user: userEvent.setup(), unmount };
}

function byClass(className: string): HTMLElement {
  return screen.getByText((_, element) => element?.classList.contains(className) === true);
}

const textBlock = (): HTMLElement => byClass('reader-text');

const status = (): string | null => screen.getByRole('status').textContent;

describe('ReaderWindow keys', () => {
  it('pauses and resumes with Space, even on a focused button', () => {
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(1);
    fireEvent.keyDown(screen.getByRole('button', { name: 'Pause' }), { key: ' ' });
    expect(status()).toBe('Paused. Sentence 2 of 3.');
    fireEvent.keyDown(document.body, { key: ' ' });
    expect(rig.gateway.reads.at(-1)?.from).toBe(1);
  });

  it('ignores a held key repeating', () => {
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(1);
    const reads = rig.gateway.reads.length;
    fireEvent.keyDown(document.body, { key: '2', repeat: true });
    fireEvent.keyDown(document.body, { key: 'ArrowRight', repeat: true });
    expect(rig.gateway.reads).toHaveLength(reads);
  });

  it('moves by sentence with the arrows', () => {
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(1);
    fireEvent.keyDown(document.body, { key: 'ArrowRight' });
    expect(rig.gateway.reads.at(-1)?.from).toBe(2);
    fireEvent.keyDown(document.body, { key: 'ArrowLeft' });
    expect(rig.gateway.reads.at(-1)?.from).toBe(1);
  });

  it('picks a mood with a number and says so on the mood button', () => {
    const rig = start();
    rig.gateway.select();
    fireEvent.keyDown(document.body, { key: '5' });
    expect(rig.gateway.reads.at(-1)).toEqual({ mood: 'urgent', voice: null, from: 0 });
    expect(screen.getByRole('button', { name: 'Mood: Urgent' }).textContent).toBe('Urgent');
  });

  it('sizes the text with Ctrl and plus or minus', () => {
    const rig = start();
    rig.gateway.select();
    expect(textBlock().style.fontSize).toBe('25px');
    fireEvent.keyDown(document.body, { key: '=', ctrlKey: true });
    expect(textBlock().style.fontSize).toBe('28px');
    fireEvent.keyDown(document.body, { key: '-', ctrlKey: true });
    fireEvent.keyDown(document.body, { key: '-', ctrlKey: true });
    expect(textBlock().style.fontSize).toBe('22px');
  });

  it('stops and closes with Escape', async () => {
    const rig = start();
    rig.gateway.select();
    fireEvent.keyDown(document.body, { key: 'Escape' });
    await act(async () => {
      await Promise.resolve();
    });
    expect(rig.gateway.calls.slice(-2)).toEqual(['stop', 'dismiss']);
  });

  it('claims the keys it handles and leaves the rest', () => {
    const rig = start();
    rig.gateway.select();
    expect(fireEvent.keyDown(document.body, { key: 'ArrowRight' })).toBe(false);
    expect(fireEvent.keyDown(document.body, { key: 'Enter' })).toBe(true);
  });

  it('leaves keys alone in a select, after another handler, and once closed', async () => {
    const rig = start();
    rig.gateway.select();
    await rig.user.click(screen.getByRole('button', { name: 'Voice: Default voice' }));
    fireEvent.keyDown(await screen.findByRole('combobox', { name: 'Voice' }), {
      key: 'ArrowRight',
    });
    const handled = new KeyboardEvent('keydown', { key: 'ArrowRight', cancelable: true });
    handled.preventDefault();
    document.body.dispatchEvent(handled);
    rig.unmount();
    fireEvent.keyDown(document.body, { key: 'ArrowRight' });
    expect(rig.gateway.reads).toHaveLength(1);
  });

  it('leaves Space to a focused mood and closes the menu first with Escape', async () => {
    const rig = start();
    rig.gateway.select();
    await rig.user.click(screen.getByRole('button', { name: 'Mood: Neutral' }));
    const calm = screen.getByRole('menuitemradio', { name: /Calm/ });
    expect(fireEvent.keyDown(calm, { key: ' ' })).toBe(true);
    expect(rig.gateway.calls).toEqual(['read']);
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(rig.gateway.calls).toEqual(['read']);
  });
});

describe('ReaderWindow keys in the mood menu', () => {
  it('picks a mood with a number while the menu has focus', async () => {
    const rig = start();
    rig.gateway.select();
    await rig.user.click(screen.getByRole('button', { name: 'Mood: Neutral' }));
    fireEvent.keyDown(screen.getByRole('menuitemradio', { name: /Calm/ }), { key: '5' });
    expect(rig.gateway.reads.at(-1)).toEqual({ mood: 'urgent', voice: null, from: 0 });
  });
});

describe('ReaderWindow mood and voice buttons', () => {
  it('chooses a mood from the mood button', async () => {
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(1);
    const moodButton = screen.getByRole('button', { name: 'Mood: Neutral' });
    expect(moodButton.getAttribute('aria-expanded')).toBe('false');
    await rig.user.click(moodButton);
    expect(moodButton.getAttribute('aria-expanded')).toBe('true');
    await rig.user.click(screen.getByRole('menuitemradio', { name: /Whisper/ }));
    expect(rig.gateway.reads.at(-1)).toEqual({ mood: 'whisper', voice: null, from: 1 });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(screen.getByRole('button', { name: 'Mood: Whisper' }).textContent).toBe('Whisper');
  });

  it('picks a voice from the voice button and names it', async () => {
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(2);
    await rig.user.click(screen.getByRole('button', { name: 'Voice: Default voice' }));
    await rig.user.selectOptions(
      await screen.findByRole('combobox', { name: 'Voice' }),
      'en_US-amy-medium',
    );
    expect(rig.gateway.reads.at(-1)).toEqual({
      mood: 'neutral',
      voice: 'en_US-amy-medium',
      from: 2,
    });
    expect(screen.getByRole('button', { name: 'Voice: amy (medium)' }).textContent).toBe(
      'amy (medium)',
    );
  });

  it('closes the voice picker with Escape on a select', async () => {
    const rig = start();
    rig.gateway.select();
    await rig.user.click(screen.getByRole('button', { name: 'Voice: Default voice' }));
    fireEvent.keyDown(await screen.findByRole('combobox', { name: 'Voice' }), { key: 'Escape' });
    expect(screen.queryByRole('combobox', { name: 'Voice' })).toBeNull();
    expect(rig.gateway.calls).toEqual(['read']);
  });
});
