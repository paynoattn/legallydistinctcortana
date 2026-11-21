import { CURRENT_SELECTION_COMMAND } from './read-aloud-ipc-names.ts';
import { TauriReadAloudGateway } from './tauri-read-aloud-gateway.ts';

type Handler = (event: { readonly payload: unknown }) => void;

function settle<T>(value: T, failure: unknown): Promise<T> {
  const { promise, reject, resolve } = Promise.withResolvers<T>();
  if (failure === undefined) {
    resolve(value);
  } else {
    reject(failure);
  }
  return promise;
}

export class FakeTauri {
  readonly invocations: { command: string; args: Record<string, unknown> | undefined }[] = [];
  readonly handlers = new Map<string, Handler>();
  invokeFailure: unknown = undefined;
  listenFailure: unknown = undefined;
  currentSelection: unknown = null;
  currentSelectionFailure: unknown = undefined;
  currentGate: PromiseWithResolvers<unknown> | undefined = undefined;
  listenGate: PromiseWithResolvers<undefined> | undefined = undefined;

  readonly invoke = (command: string, args?: Record<string, unknown>): Promise<unknown> => {
    this.invocations.push({ command, args });
    if (command === CURRENT_SELECTION_COMMAND) {
      if (this.currentGate !== undefined) {
        return this.currentGate.promise;
      }
      return settle(this.currentSelection, this.currentSelectionFailure);
    }
    return settle(null, this.invokeFailure);
  };

  readonly listen = (event: string, handler: Handler): Promise<unknown> => {
    if (this.listenFailure !== undefined) {
      return settle(null, this.listenFailure);
    }
    this.handlers.set(event, handler);
    if (this.listenGate !== undefined) {
      return this.listenGate.promise;
    }
    return Promise.resolve(() => undefined);
  };

  emit(event: string, payload: unknown): void {
    const handler = this.handlers.get(event);
    if (handler === undefined) {
      throw new Error(event);
    }
    handler({ payload });
  }
}

export function start(): { tauri: FakeTauri; gateway: TauriReadAloudGateway } {
  const tauri = new FakeTauri();
  return { tauri, gateway: new TauriReadAloudGateway(tauri.invoke, tauri.listen) };
}

export const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));
