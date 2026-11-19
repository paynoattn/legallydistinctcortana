import type { Draft } from '../domain/settings.ts';

export function errorIdFor(name: keyof Draft): string {
  return `${name}-error`;
}

export function describedBy(name: keyof Draft, error: string | undefined): string | undefined {
  return error === undefined ? undefined : errorIdFor(name);
}
