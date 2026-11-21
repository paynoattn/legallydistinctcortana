import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { FAKE_VOICES } from '../infrastructure/fake-read-aloud-ipc.ts';
import { focused, stores, type Stores } from './reader-window.fixture.ts';
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

function sentenceAt(index: number): HTMLElement {
  return screen.getByText((_, element) => element?.getAttribute('data-sentence') === String(index));
}

function litWords(): string[] {
  return screen
    .queryAllByText((_, element) => element?.classList.contains('is-lit') === true)
    .map((word) => word.textContent);
}

describe('ReaderWindow before reading', () => {
  it('asks for a selection and focuses Close while waiting', () => {
    start();
    expect(screen.getByRole('heading', { name: 'Read aloud' })).toBeTruthy();
    expect(
      screen.getByText('Select text in any app, then press the read-aloud shortcut.'),
    ).toBeTruthy();
    expect(focused(screen.getByRole('button', { name: 'Close' }))).toBe(true);
  });

  it('shows why nothing can be read, focuses Close, and reads nothing', async () => {
    const rig = start();
    act(() => {
      rig.gateway.emitSelection({ kind: 'failed', message: 'Select some text first.' });
    });
    expect(screen.getByRole('alert').textContent).toBe('Select some text first.');
    expect(focused(screen.getByRole('button', { name: 'Close' }))).toBe(true);
    await rig.user.click(screen.getByRole('button', { name: 'Close' }));
    expect(rig.gateway.reads).toEqual([]);
    expect(rig.gateway.calls).toEqual(['stop', 'dismiss']);
  });
});

function progressFill(): string {
  const bar = screen.getByRole('progressbar', { name: 'Sentences read' });
  return within(bar).getByText((_, element) => element?.tagName === 'I').style.width;
}

function recordFocus(): { readonly names: string[]; readonly stop: () => void } {
  const names: string[] = [];
  const onFocus = (event: FocusEvent): void => {
    names.push(
      event.target instanceof Element ? (event.target.getAttribute('aria-label') ?? '') : '',
    );
  };
  document.addEventListener('focusin', onFocus);
  return {
    names,
    stop: () => {
      document.removeEventListener('focusin', onFocus);
    },
  };
}

describe('ReaderWindow reading', () => {
  it('moves focus straight to Pause when text arrives, never through Close', () => {
    const rig = start();
    const focus = recordFocus();
    rig.gateway.select();
    focus.stop();
    expect(focus.names).toEqual(['Pause']);
  });

  it('renders the words without React complaining about their keys', () => {
    const errors = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(1);
    expect(errors).not.toHaveBeenCalled();
    errors.mockRestore();
  });

  it('puts the light over the spoken word only', () => {
    const widths = vi
      .spyOn(HTMLElement.prototype, 'offsetWidth', 'get')
      .mockImplementation(function measured(this: HTMLElement) {
        return this.textContent.length * 10;
      });
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(1);
    expect(byClass('reader-light').style.width).toBe('40px');
    widths.mockRestore();
  });

  it('explains the keys under the controls', () => {
    const rig = start();
    rig.gateway.select();
    expect(byClass('reader-hint').textContent).toBe(
      'Space pauses. ← → skip sentences. 1–9 pick a mood. Click a word to go there. Esc closes.',
    );
  });

  it('fills the progress line by the share of sentences done', () => {
    const rig = start();
    rig.gateway.select();
    expect(progressFill()).toBe('0%');
    rig.gateway.startSentence(2);
    expect(progressFill()).toBe('66.6667%');
    rig.gateway.report({ kind: 'finished' });
    expect(progressFill()).toBe('100%');
  });

  it('leaves the progress line empty for a selection with no sentences', () => {
    const rig = start();
    rig.gateway.select([]);
    expect(progressFill()).toBe('0%');
  });

  it('shows the text as paragraphs of sentences of words and starts reading', () => {
    const rig = start();
    rig.gateway.select();
    const paragraphs = within(textBlock()).getAllByRole('paragraph');
    expect([...paragraphs].map((paragraph) => paragraph.textContent.trim())).toEqual([
      'Good morning.',
      'Two emails need you. Lunch is at noon.',
    ]);
    expect(within(sentenceAt(1)).getByText('emails').className).toBe('word');
    expect(textBlock().getAttribute('aria-live')).toBe('off');
    expect(rig.gateway.reads).toEqual([{ mood: 'neutral', voice: null, from: 0 }]);
    expect(status()).toBe('Sentence 1 of 3.');
    expect(focused(screen.getByRole('button', { name: 'Pause' }))).toBe(true);
  });

  it('lights the spoken word and colours the sentences around it', () => {
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(1);
    expect(litWords()).toEqual(['Two']);
    expect([0, 1, 2].map((index) => sentenceAt(index).dataset['state'])).toEqual([
      'read',
      'now',
      'ahead',
    ]);
    expect(byClass('reader-light').className).toBe('reader-light');
    expect(status()).toBe('Sentence 2 of 3.');
  });

  it('shows the light only while a word is lit', () => {
    const rig = start();
    rig.gateway.select();
    expect(byClass('reader-light').className).toBe('reader-light is-off');
    rig.gateway.startSentence(0);
    rig.gateway.report({ kind: 'finished' });
    expect(litWords()).toEqual([]);
    expect(byClass('reader-light').className).toBe('reader-light is-off');
  });

  it('shows the sentences done and dims the line when finished', () => {
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(2);
    const progress = screen.getByRole('progressbar', { name: 'Sentences read' });
    expect(progress.getAttribute('aria-valuenow')).toBe('2');
    expect(progress.getAttribute('aria-valuemax')).toBe('3');
    expect(progress.className).toBe('reader-progress');
    rig.gateway.report({ kind: 'finished' });
    expect(progress.getAttribute('aria-valuenow')).toBe('3');
    expect(progress.className).toBe('reader-progress is-done');
    expect(status()).toBe('Finished reading.');
    expect(screen.getByRole('button', { name: 'Play' })).toBeTruthy();
  });

  it('reads from the sentence of a clicked word, and ignores clicks between paragraphs', async () => {
    const rig = start();
    rig.gateway.select();
    await rig.user.click(within(sentenceAt(2)).getByText('noon.'));
    expect(rig.gateway.reads.at(-1)?.from).toBe(2);
    await rig.user.click(textBlock());
    expect(rig.gateway.reads).toHaveLength(2);
  });

  it('pauses and plays with the button', async () => {
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(1);
    await rig.user.click(screen.getByRole('button', { name: 'Pause' }));
    expect(status()).toBe('Paused. Sentence 2 of 3.');
    await rig.user.click(screen.getByRole('button', { name: 'Play' }));
    expect(rig.gateway.reads.at(-1)?.from).toBe(1);
  });

  it('steps through the speeds with the speed button, restarting the sentence', async () => {
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(1);
    await rig.user.click(screen.getByRole('button', { name: 'Speed: 1×' }));
    expect(screen.getByRole('button', { name: 'Speed: 1.25×' }).textContent).toBe('1.25×');
    expect(rig.gateway.reads.at(-1)?.from).toBe(1);
  });

  it('moves by sentence with the buttons', async () => {
    const rig = start();
    rig.gateway.select();
    rig.gateway.startSentence(1);
    await rig.user.click(screen.getByRole('button', { name: 'Next sentence' }));
    expect(rig.gateway.reads.at(-1)?.from).toBe(2);
    await rig.user.click(screen.getByRole('button', { name: 'Previous sentence' }));
    expect(rig.gateway.reads.at(-1)?.from).toBe(1);
  });
});

describe('ReaderWindow following the voice', () => {
  it('stops following on the wheel and resumes from the button', async () => {
    const rig = start();
    rig.gateway.select();
    expect(screen.queryByRole('button', { name: 'Follow the voice' })).toBeNull();
    const scroller = byClass('reader-scroller');
    fireEvent.wheel(scroller);
    await rig.user.click(screen.getByRole('button', { name: 'Follow the voice' }));
    expect(screen.queryByRole('button', { name: 'Follow the voice' })).toBeNull();
  });

  it('stops following when the scrollbar is grabbed, not when a word is pressed', () => {
    const rig = start();
    rig.gateway.select();
    const scroller = byClass('reader-scroller');
    fireEvent.pointerDown(within(sentenceAt(0)).getByText('Good'));
    expect(rig.store.getSnapshot().following).toBe(true);
    fireEvent.pointerDown(scroller);
    expect(rig.store.getSnapshot().following).toBe(false);
  });
});
