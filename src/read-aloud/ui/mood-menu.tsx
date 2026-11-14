import type { KeyboardEvent, ReactElement } from 'react';
import { nextFocusIndex } from '../domain/menu-navigation.ts';
import { MOODS, type MoodId, moodForKey } from '../domain/mood.ts';
import { focusOnMount } from './focus-on-mount.ts';

interface MoodMenuProps {
  readonly onChoose: (mood: MoodId) => void;
  readonly onDismiss: () => void;
}

function hasModifier(event: KeyboardEvent<HTMLDivElement>): boolean {
  return event.ctrlKey || event.altKey || event.metaKey;
}

function moveFocus(event: KeyboardEvent<HTMLDivElement>): void {
  const items = [...event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')];
  const current = items.findIndex((item) => item === document.activeElement);
  const next = nextFocusIndex(event.key, current, items.length);
  if (next !== undefined) {
    event.preventDefault();
    items[next]?.focus();
  }
}

export function MoodMenu({ onChoose, onDismiss }: MoodMenuProps): ReactElement {
  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>): void => {
    const mood = hasModifier(event) ? undefined : moodForKey(event.key);
    if (event.key === 'Escape') {
      onDismiss();
    } else if (mood === undefined) {
      moveFocus(event);
    } else {
      onChoose(mood.id);
    }
  };
  return (
    <div role="menu" aria-label="Read in a mood" tabIndex={-1} onKeyDown={handleKeyDown}>
      {MOODS.map((mood, index) => (
        <button
          key={mood.id}
          type="button"
          role="menuitem"
          tabIndex={index === 0 ? 0 : -1}
          aria-keyshortcuts={String(index + 1)}
          ref={index === 0 ? focusOnMount : undefined}
          onClick={() => {
            onChoose(mood.id);
          }}
        >
          <kbd aria-hidden="true">{index + 1}</kbd>
          <span className="mood-label">{mood.label}</span>
          <span className="mood-hint">{mood.hint}</span>
        </button>
      ))}
    </div>
  );
}
