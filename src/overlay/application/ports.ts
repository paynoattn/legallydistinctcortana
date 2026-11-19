import type { OverlayUpdate } from '../domain/overlay-state.ts';

export interface OverlayListeners {
  readonly state: (update: OverlayUpdate) => void;
  readonly level: (level: number) => void;
}

export interface OverlayGateway {
  listen(listeners: OverlayListeners): void;
  fix(): Promise<void>;
  dismiss(): Promise<void>;
}

export interface MotionPreference {
  prefersReduced(): boolean;
  onChange(listener: () => void): void;
}
