import type { ReaderStore } from '../application/reader-store.ts';
import { commandFor, type ReaderCommand } from '../domain/reader-keys.ts';

function isField(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLSelectElement ||
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement
  );
}

function ignored(event: KeyboardEvent): boolean {
  return event.repeat || event.defaultPrevented || isField(event.target);
}

function inMenu(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest('[role="menu"]') !== null;
}

const ACTIONS: Readonly<
  Record<Exclude<ReaderCommand['kind'], 'mood'>, (store: ReaderStore) => void>
> = {
  toggle: (store) => {
    store.togglePlay();
  },
  previous: (store) => {
    store.previous();
  },
  next: (store) => {
    store.next();
  },
  larger: (store) => {
    store.larger();
  },
  smaller: (store) => {
    store.smaller();
  },
  close: (store) => {
    store.escape();
  },
};

function run(store: ReaderStore, command: ReaderCommand): void {
  if (command.kind === 'mood') {
    store.chooseMood(command.mood);
  } else {
    ACTIONS[command.kind](store);
  }
}

export function readerKeys(
  store: ReaderStore,
): (root: HTMLElement | null) => (() => void) | undefined {
  return (root) => {
    const document = root?.ownerDocument;
    if (document === undefined) {
      return undefined;
    }
    const onKeyDown = (event: KeyboardEvent): void => {
      const command = ignored(event) ? undefined : commandFor(event);
      if (command === undefined || (command.kind === 'toggle' && inMenu(event.target))) {
        return;
      }
      event.preventDefault();
      run(store, command);
    };
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('keydown', onKeyDown);
    };
  };
}
