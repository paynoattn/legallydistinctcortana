import type { Connection } from '../domain/connection.ts';
import type { FieldErrors, Settings } from '../domain/settings.ts';

export type SaveResult =
  | { readonly kind: 'saved'; readonly settings: Settings }
  | { readonly kind: 'rejected'; readonly errors: FieldErrors };

export interface SettingsGateway {
  load(): Promise<Settings>;
  defaults(): Promise<Settings>;
  save(settings: Settings): Promise<SaveResult>;
  connectionStatus(): Promise<Connection>;
  checkConnection(): Promise<Connection>;
  onConnection(listener: (connection: Connection) => void): void;
  suspendShortcuts(): Promise<void>;
  resumeShortcuts(): Promise<void>;
}
