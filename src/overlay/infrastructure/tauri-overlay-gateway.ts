import type { OverlayGateway, OverlayListeners } from '../application/ports.ts';
import {
  failed,
  type Fix,
  type OverlayState,
  type OverlayUpdate,
} from '../domain/overlay-state.ts';

export type Invoke = (command: string) => Promise<unknown>;
export type Listen = (
  event: string,
  handler: (event: { readonly payload: unknown }) => void,
) => Promise<unknown>;

export const STATE_EVENT = 'overlay:state';
export const LEVEL_EVENT = 'overlay:level';
export const FIX_COMMAND = 'overlay_fix';
export const DISMISS_COMMAND = 'overlay_dismiss';
export const UNEXPECTED_MESSAGE = 'The app sent an update this window does not understand.';

type PlainKind = Exclude<OverlayState['kind'], 'error'>;

const PLAIN_KINDS: readonly PlainKind[] = [
  'hidden',
  'armed',
  'listening',
  'thinking',
  'inserted',
  'speaking',
];

function field(value: unknown, key: string): unknown {
  return typeof value === 'object' && value !== null ? Reflect.get(value, key) : undefined;
}

function isPlainKind(kind: unknown): kind is PlainKind {
  return PLAIN_KINDS.some((plain) => plain === kind);
}

function parseFix(value: unknown): Fix | null | undefined {
  return value === null || value === 'sound_settings' || value === 'settings' ? value : undefined;
}

function parseError(value: unknown): OverlayState | null {
  const reason = field(value, 'reason');
  const fix = parseFix(field(value, 'fix'));
  return typeof reason === 'string' && fix !== undefined ? { kind: 'error', reason, fix } : null;
}

function parseState(value: unknown): OverlayState | null {
  const kind = field(value, 'kind');
  if (kind === 'error') {
    return parseError(value);
  }
  return isPlainKind(kind) ? { kind } : null;
}

export function parseUpdate(payload: unknown): OverlayUpdate | null {
  const state = parseState(field(payload, 'state'));
  const reduceMotion = field(payload, 'reduceMotion');
  return state !== null && typeof reduceMotion === 'boolean' ? { state, reduceMotion } : null;
}

export function parseLevel(payload: unknown): number | null {
  const level = field(payload, 'level');
  return typeof level === 'number' && Number.isFinite(level) ? level : null;
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export class TauriOverlayGateway implements OverlayGateway {
  readonly #invoke: Invoke;
  readonly #listen: Listen;

  constructor(invoke: Invoke, listen: Listen) {
    this.#invoke = invoke;
    this.#listen = listen;
  }

  listen({ state, level }: OverlayListeners): void {
    const fail = (error: unknown): void => {
      state(failed(messageOf(error)));
    };
    this.#listen(STATE_EVENT, ({ payload }) => {
      state(parseUpdate(payload) ?? failed(UNEXPECTED_MESSAGE));
    }).catch(fail);
    this.#listen(LEVEL_EVENT, ({ payload }) => {
      const parsed = parseLevel(payload);
      if (parsed === null) {
        state(failed(UNEXPECTED_MESSAGE));
      } else {
        level(parsed);
      }
    }).catch(fail);
  }

  async fix(): Promise<void> {
    await this.#invoke(FIX_COMMAND);
  }

  async dismiss(): Promise<void> {
    await this.#invoke(DISMISS_COMMAND);
  }
}
