const FAKE_EVENT_PREFIX = 'fake-ipc:';
export const INVOKED_EVENT = 'fake-ipc:invoked';

export class WindowEventIpc {
  readonly #target: EventTarget;

  constructor(target: EventTarget) {
    this.#target = target;
  }

  readonly invoke = (command: string): Promise<unknown> => {
    this.#target.dispatchEvent(new CustomEvent(INVOKED_EVENT, { detail: command }));
    return Promise.resolve(null);
  };

  readonly listen = (
    event: string,
    handler: (event: { readonly payload: unknown }) => void,
  ): Promise<unknown> => {
    this.#target.addEventListener(`${FAKE_EVENT_PREFIX}${event}`, (fired) => {
      handler({ payload: fired instanceof CustomEvent ? fired.detail : undefined });
    });
    return Promise.resolve(null);
  };
}
