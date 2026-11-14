import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { MOODS, type MoodId } from '../domain/mood.ts';
import { MoodMenu } from './mood-menu.tsx';

function startMenu(): { chosen: MoodId[]; dismissals: number[] } {
  const chosen: MoodId[] = [];
  const dismissals: number[] = [];
  render(
    <MoodMenu
      onChoose={(mood) => chosen.push(mood)}
      onDismiss={() => dismissals.push(dismissals.length)}
    />,
  );
  return { chosen, dismissals };
}

const items = (): HTMLElement[] => screen.getAllByRole('menuitem');
const first = (): HTMLElement => screen.getByRole('menuitem', { name: /Neutral/ });

describe('MoodMenu', () => {
  it.each(['ctrlKey', 'altKey', 'metaKey'])(
    'ignores the number keys while %s is held',
    (modifier) => {
      const { chosen } = startMenu();
      fireEvent.keyDown(first(), { key: '3', [modifier]: true });
      expect(chosen).toEqual([]);
      fireEvent.keyDown(first(), { key: '3' });
      expect(chosen).toEqual(['cheerful']);
    },
  );

  it('leaves a held modifier with an arrow key to the navigation', () => {
    startMenu();
    expect(fireEvent.keyDown(first(), { key: 'ArrowDown', ctrlKey: true })).toBe(false);
  });

  it('takes over the arrow keys but leaves other keys alone', () => {
    startMenu();
    expect(fireEvent.keyDown(first(), { key: 'ArrowDown' })).toBe(false);
    expect(fireEvent.keyDown(first(), { key: 'x' })).toBe(true);
  });

  it('dismisses on Escape and chooses nothing', () => {
    const { chosen, dismissals } = startMenu();
    fireEvent.keyDown(first(), { key: 'Escape' });
    expect(dismissals).toHaveLength(1);
    expect(chosen).toEqual([]);
  });

  it('makes only the first mood a tab stop, and not the menu itself', () => {
    startMenu();
    expect(items().map((item) => item.tabIndex)).toEqual([0, -1, -1, -1, -1, -1, -1, -1, -1]);
    expect(screen.getByRole('menu').tabIndex).toBe(-1);
  });

  it('announces each shortcut and hides the number badge from assistive tech', () => {
    startMenu();
    expect(items().map((item) => item.getAttribute('aria-keyshortcuts'))).toEqual(
      MOODS.map((_, index) => String(index + 1)),
    );
    items().forEach((item, index) => {
      const badge = within(item).getByText(String(index + 1));
      expect(badge.getAttribute('aria-hidden')).toBe('true');
    });
  });
});
