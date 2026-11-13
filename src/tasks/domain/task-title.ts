export const MAX_TITLE_LENGTH = 200;

declare const brand: unique symbol;

export type TaskTitle = string & { readonly [brand]: 'TaskTitle' };

export type TitleError = 'empty' | 'too-long';

export type TitleResult =
  | { readonly ok: true; readonly title: TaskTitle }
  | { readonly ok: false; readonly error: TitleError };

export function isTaskTitle(value: unknown): value is TaskTitle {
  return (
    typeof value === 'string' &&
    value === value.trim() &&
    value.length > 0 &&
    value.length <= MAX_TITLE_LENGTH
  );
}

export function parseTaskTitle(raw: string): TitleResult {
  const trimmed = raw.trim();
  if (isTaskTitle(trimmed)) {
    return { ok: true, title: trimmed };
  }
  return { ok: false, error: trimmed.length === 0 ? 'empty' : 'too-long' };
}
