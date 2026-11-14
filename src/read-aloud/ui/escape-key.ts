import type { KeyboardEvent } from 'react';

export function onEscape(handler: () => void): (event: KeyboardEvent<HTMLElement>) => void {
  return (event) => {
    if (event.key === 'Escape') {
      handler();
    }
  };
}
