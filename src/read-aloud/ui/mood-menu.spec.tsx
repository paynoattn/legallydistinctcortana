import { fireEvent, render, screen, within } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { MOODS, type MoodId } from '../domain/mood.ts';
import { MoodMenu } from './mood-menu.tsx';

function startMenu(current: MoodId = 'neutral'): { chosen: MoodId[]; closes: number[] } {
  const chosen: MoodId[] = [];
  const closes: number[] = [];
  render(
    <MoodMenu
      current={current}
      onChoose={(mood) => chosen.push(mood)}
      onClose={() => closes.push(closes.length)}
    />,
  );
  return { chosen, closes };
}

const items = (): HTMLElement[] => screen.getAllByRole('menuitemradio');
const item = (name: RegExp): HTMLElement => screen.getByRole('menuitemradio', { name });
const focused = (element: HTMLElement): boolean => element.matches(':focus');

describe('MoodMenu', () => {
  it('lists the moods in order and checks the current one', () => {
    startMenu('calm');
    expect(items().map((entry) => entry.textContent)).toEqual(
      MOODS.map((mood, index) => `${String(index + 1)}${mood.label}${mood.hint}`),
    );
    expect(items().map((entry) => entry.getAttribute('aria-checked'))).toEqual([
      'false',
      'true',
      'false',
      'false',
      'false',
      'false',
      'false',
      'false',
      'false',
    ]);
  });

  it('focuses the current mood, the only tab stop', () => {
    startMenu('urgent');
    expect(focused(item(/Urgent/))).toBe(true);
    expect(items().map((entry) => entry.tabIndex)).toEqual([-1, -1, -1, -1, 0, -1, -1, -1, -1]);
    expect(screen.getByRole('menu').tabIndex).toBe(-1);
  });

  it('chooses a mood by click', async () => {
    const { chosen } = startMenu();
    await userEvent.setup().click(item(/Whisper/));
    expect(chosen).toEqual(['whisper']);
  });

  it('moves focus with the arrow keys and keeps them from the reader', () => {
    startMenu();
    const outside: string[] = [];
    document.addEventListener('keydown', (event) => outside.push(event.key), { once: true });
    expect(fireEvent.keyDown(item(/Neutral/), { key: 'ArrowDown' })).toBe(false);
    expect(focused(item(/Calm/))).toBe(true);
    expect(outside).toEqual([]);
  });

  it('moves focus from the focused mood, not from the top', () => {
    startMenu('urgent');
    fireEvent.keyDown(item(/Urgent/), { key: 'ArrowDown' });
    expect(focused(item(/Clipped and forceful/))).toBe(true);
  });

  it('leaves other keys to the reader', () => {
    startMenu();
    expect(fireEvent.keyDown(item(/Neutral/), { key: '3' })).toBe(true);
  });

  it('closes on Escape without closing the reader', () => {
    const { chosen, closes } = startMenu();
    const outside: string[] = [];
    document.addEventListener('keydown', (event) => outside.push(event.key), { once: true });
    fireEvent.keyDown(item(/Neutral/), { key: 'Escape' });
    expect(closes).toHaveLength(1);
    expect(chosen).toEqual([]);
    expect(outside).toEqual([]);
  });

  it('announces each shortcut and hides the number badge from assistive tech', () => {
    startMenu();
    expect(items().map((entry) => entry.getAttribute('aria-keyshortcuts'))).toEqual(
      MOODS.map((_, index) => String(index + 1)),
    );
    items().forEach((entry, index) => {
      const badge = within(entry).getByText(String(index + 1));
      expect(badge.getAttribute('aria-hidden')).toBe('true');
    });
  });
});
