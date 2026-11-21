import type { KeyboardEvent, ReactElement } from 'react';
import { nextFocusIndex } from '../domain/menu-navigation.ts';
import { MOODS, type MoodId } from '../domain/mood.ts';
import { focusOnMount } from './focus-on-mount.ts';

interface MoodMenuProps {
  readonly current: MoodId;
  readonly onChoose: (mood: MoodId) => void;
  readonly onClose: () => void;
}

function moveFocus(event: KeyboardEvent<HTMLDivElement>): void {
  const items = [
    ...event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitemradio"]'),
  ];
  const current = items.findIndex((item) => item === document.activeElement);
  const next = nextFocusIndex(event.key, current, items.length);
  if (next !== undefined) {
    event.preventDefault();
    event.stopPropagation();
    items[next]?.focus();
  }
}

export function MoodMenu({ current, onChoose, onClose }: MoodMenuProps): ReactElement {
  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>): void => {
    if (event.key === 'Escape') {
      event.stopPropagation();
      onClose();
    } else {
      moveFocus(event);
    }
  };
  return (
    <div
      className="mood-menu"
      role="menu"
      aria-label="Mood"
      tabIndex={-1}
      onKeyDown={handleKeyDown}
    >
      {MOODS.map((mood, index) => (
        <button
          key={mood.id}
          type="button"
          role="menuitemradio"
          aria-checked={mood.id === current}
          tabIndex={mood.id === current ? 0 : -1}
          aria-keyshortcuts={String(index + 1)}
          ref={mood.id === current ? focusOnMount : undefined}
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
