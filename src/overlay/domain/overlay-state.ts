export type Fix = 'sound_settings' | 'settings';

export type OverlayState =
  | { readonly kind: 'hidden' }
  | { readonly kind: 'armed' }
  | { readonly kind: 'listening' }
  | { readonly kind: 'thinking' }
  | { readonly kind: 'inserted' }
  | { readonly kind: 'speaking' }
  | { readonly kind: 'error'; readonly reason: string; readonly fix: Fix | null };

export type OverlayKind = OverlayState['kind'];

export interface OverlayUpdate {
  readonly state: OverlayState;
  readonly reduceMotion: boolean;
}

export const HIDDEN: OverlayState = { kind: 'hidden' };

export function fixLabel(fix: Fix): string {
  return fix === 'sound_settings' ? 'Choose a mic' : 'Open settings';
}

export function failed(reason: string): OverlayUpdate {
  return { state: { kind: 'error', reason, fix: null }, reduceMotion: false };
}
