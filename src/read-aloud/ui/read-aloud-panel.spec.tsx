import { act, render, screen } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { ReadAloudStore } from '../application/read-aloud-store.ts';
import type { ReadAloudGateway } from '../application/ports.ts';
import { MOODS, type MoodId } from '../domain/mood.ts';
import type { ReadingStatus, Selection } from '../domain/read-aloud-state.ts';
import { ReadAloudPanel } from './read-aloud-panel.tsx';

class ScriptedGateway implements ReadAloudGateway {
  readonly calls: string[] = [];
  failWith: unknown = undefined;
  emitSelection: (selection: Selection) => void = () => undefined;
  emitStatus: (status: ReadingStatus) => void = () => undefined;

  onSelection(listener: (selection: Selection) => void): void {
    this.emitSelection = listener;
  }

  onStatus(listener: (status: ReadingStatus) => void): void {
    this.emitStatus = listener;
  }

  read(mood: MoodId): Promise<void> {
    this.calls.push(`read ${mood}`);
    return this.outcome();
  }

  stop(): Promise<void> {
    this.calls.push('stop');
    return this.outcome();
  }

  dismiss(): Promise<void> {
    this.calls.push('dismiss');
    return this.outcome();
  }

  outcome(): Promise<void> {
    const { promise, reject, resolve } = Promise.withResolvers<undefined>();
    if (this.failWith === undefined) {
      resolve(undefined);
    } else {
      reject(this.failWith);
    }
    return promise;
  }
}

function startPanel(): { gateway: ScriptedGateway; user: ReturnType<typeof userEvent.setup> } {
  const gateway = new ScriptedGateway();
  render(<ReadAloudPanel store={new ReadAloudStore(gateway)} />);
  return { gateway, user: userEvent.setup() };
}

function select(gateway: ScriptedGateway, selection: Selection): void {
  act(() => {
    gateway.emitSelection(selection);
  });
}

function emitStatus(gateway: ScriptedGateway, status: ReadingStatus): void {
  act(() => {
    gateway.emitStatus(status);
  });
}

const READY: Selection = { kind: 'ready', preview: 'Hello there world', charCount: 17 };

function startReady(): ReturnType<typeof startPanel> {
  const started = startPanel();
  select(started.gateway, READY);
  return started;
}
function isFocused(element: HTMLElement | undefined): boolean {
  return element?.matches(':focus') ?? false;
}

const items = (): HTMLElement[] => screen.getAllByRole('menuitem');

describe('ReadAloudPanel waiting', () => {
  it('prompts for a selection and offers Close, focused', () => {
    startPanel();
    expect(screen.getByRole('heading', { name: 'Read aloud' })).toBeTruthy();
    expect(
      screen.getByText('Select text in any app, then press the read-aloud shortcut.'),
    ).toBeTruthy();
    expect(isFocused(screen.getByRole('button', { name: 'Close' }))).toBe(true);
    expect(screen.queryByRole('menu')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('dismisses with the Close button', async () => {
    const { gateway, user } = startPanel();
    await user.click(screen.getByRole('button', { name: 'Close' }));
    expect(gateway.calls).toEqual(['dismiss']);
  });

  it('dismisses with Escape', async () => {
    const { gateway, user } = startPanel();
    await user.keyboard('{Escape}');
    expect(gateway.calls).toEqual(['dismiss']);
  });

  it('ignores other keys on Close', async () => {
    const { gateway, user } = startPanel();
    await user.keyboard('{Enter}');
    await user.keyboard('a');
    expect(gateway.calls).toEqual(['dismiss']);
  });
});

describe('ReadAloudPanel nothing selected', () => {
  const failed: Selection = { kind: 'failed', message: 'Select some text first.' };

  it('shows the message as an alert with a focused Close button', () => {
    const { gateway } = startPanel();
    select(gateway, failed);
    expect(screen.getByRole('alert').textContent).toBe('Select some text first.');
    expect(
      screen.queryByText('Select text in any app, then press the read-aloud shortcut.'),
    ).toBeNull();
    expect(isFocused(screen.getByRole('button', { name: 'Close' }))).toBe(true);
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('dismisses with Escape or click', async () => {
    const { gateway, user } = startPanel();
    select(gateway, failed);
    await user.keyboard('{Escape}');
    await user.click(screen.getByRole('button', { name: 'Close' }));
    expect(gateway.calls).toEqual(['dismiss', 'dismiss']);
  });
});

describe('ReadAloudPanel with a selection', () => {
  it('shows the preview and the character count', () => {
    startReady();
    expect(screen.getByText('Hello there world')).toBeTruthy();
    expect(screen.getByText('17 characters')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Close' })).toBeNull();
  });

  it('uses the singular for one character', () => {
    const { gateway } = startPanel();
    select(gateway, { kind: 'ready', preview: 'a', charCount: 1 });
    expect(screen.getByText('1 character')).toBeTruthy();
  });

  it('lists the nine moods with their shortcut numbers and hints, first focused', () => {
    startReady();
    expect(screen.getByRole('menu', { name: 'Read in a mood' })).toBeTruthy();
    expect(items()).toHaveLength(9);
    items().forEach((item, index) => {
      const mood = MOODS[index];
      expect(item.textContent).toBe(`${String(index + 1)}${mood?.label ?? ''}${mood?.hint ?? ''}`);
    });
    expect(isFocused(items()[0])).toBe(true);
  });

  it('moves focus with the arrow keys, wrapping around', async () => {
    const { gateway, user } = startReady();
    await user.keyboard('{ArrowDown}');
    expect(isFocused(items()[1])).toBe(true);
    await user.keyboard('{ArrowUp}{ArrowUp}');
    expect(isFocused(items()[8])).toBe(true);
    await user.keyboard('{ArrowDown}');
    expect(isFocused(items()[0])).toBe(true);
    await user.keyboard('{End}');
    expect(isFocused(items()[8])).toBe(true);
    await user.keyboard('{Home}');
    expect(isFocused(items()[0])).toBe(true);
    expect(gateway.calls).toEqual([]);
  });

  it('keeps focus where it is on keys that mean nothing here', async () => {
    const { gateway, user } = startReady();
    await user.keyboard('{ArrowDown}x0');
    expect(isFocused(items()[1])).toBe(true);
    expect(gateway.calls).toEqual([]);
  });

  it('dismisses with Escape', async () => {
    const { gateway, user } = startReady();
    await user.keyboard('{Escape}');
    expect(gateway.calls).toEqual(['dismiss']);
  });

  it('chooses a mood by click', async () => {
    const { gateway, user } = startReady();
    await user.click(screen.getByRole('menuitem', { name: /Calm/ }));
    expect(gateway.calls).toEqual(['read calm']);
    expect(screen.getByRole('status').textContent).toBe('Reading in a calm voice. Stop');
    expect(screen.getByRole('button', { name: 'Stop' })).toBeTruthy();
  });

  it.each(MOODS.map((mood, index) => [String(index + 1), mood] as const))(
    'chooses mood %s with its number key',
    async (key, mood) => {
      const { gateway, user } = startReady();
      await user.keyboard(key);
      expect(gateway.calls).toEqual([`read ${mood.id}`]);
      expect(screen.getByRole('status').textContent).toContain(
        `Reading in a ${mood.label.toLowerCase()} voice.`,
      );
    },
  );

  it('stops reading with the Stop button', async () => {
    const { gateway, user } = startReady();
    await user.keyboard('2');
    await user.click(screen.getByRole('button', { name: 'Stop' }));
    expect(gateway.calls).toEqual(['read calm', 'stop']);
  });

  it('puts focus back on the first mood after Stop is clicked', async () => {
    const { user } = startReady();
    await user.keyboard('{ArrowDown}{ArrowDown}');
    await user.click(screen.getByRole('menuitem', { name: /Cheerful/ }));
    await user.click(screen.getByRole('button', { name: 'Stop' }));
    expect(isFocused(items()[0])).toBe(true);
  });

  it('shows the status above the preview and the menu', () => {
    const { gateway } = startReady();
    emitStatus(gateway, { kind: 'finished' });
    const status = screen.getByRole('status');
    const following = Node.DOCUMENT_POSITION_FOLLOWING;
    expect(status.compareDocumentPosition(screen.getByText('Hello there world'))).toBe(following);
    expect(status.compareDocumentPosition(screen.getByRole('menu'))).toBe(following);
  });

  it('shows the statuses the gateway reports', () => {
    const { gateway } = startReady();
    emitStatus(gateway, { kind: 'finished' });
    expect(screen.getByRole('status').textContent).toBe('Finished reading.');
    emitStatus(gateway, { kind: 'stopped' });
    expect(screen.getByRole('status').textContent).toBe('Stopped.');
    emitStatus(gateway, { kind: 'failed', message: 'Speaker unplugged.' });
    expect(screen.getByRole('alert').textContent).toBe('Speaker unplugged.');
    expect(screen.queryByRole('status')).toBeNull();
  });

  it.each([
    ['an Error', new Error('device busy'), 'device busy'],
    ['a plain string', 'device missing', 'device missing'],
  ])(
    'shows a rejected read as a failed status when it rejects with %s',
    async (_name, failure, text) => {
      const { gateway, user } = startPanel();
      gateway.failWith = failure;
      select(gateway, READY);
      await user.keyboard('1');
      expect((await screen.findByRole('alert')).textContent).toBe(text);
    },
  );

  it('shows a rejected stop as a failed status', async () => {
    const { gateway, user } = startReady();
    await user.keyboard('1');
    gateway.failWith = 'cannot stop';
    await user.click(screen.getByRole('button', { name: 'Stop' }));
    expect((await screen.findByRole('alert')).textContent).toBe('cannot stop');
  });

  it('focuses the first mood again when a new selection arrives', async () => {
    const { gateway, user } = startReady();
    await user.keyboard('{End}');
    expect(isFocused(items()[8])).toBe(true);
    select(gateway, { kind: 'ready', preview: 'Second text', charCount: 11 });
    expect(screen.getByText('Second text')).toBeTruthy();
    expect(isFocused(items()[0])).toBe(true);
  });

  it('clears the status when a new selection arrives', async () => {
    const { gateway, user } = startReady();
    await user.keyboard('1');
    select(gateway, READY);
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('returns to the waiting prompt when the selection is dismissed', () => {
    const { gateway } = startReady();
    select(gateway, { kind: 'waiting' });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(
      screen.getByText('Select text in any app, then press the read-aloud shortcut.'),
    ).toBeTruthy();
    expect(isFocused(screen.getByRole('button', { name: 'Close' }))).toBe(true);
  });
});
