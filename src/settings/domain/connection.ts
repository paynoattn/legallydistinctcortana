type ConnectionKind = 'checking' | 'connected' | 'partial' | 'unreachable' | 'failed';

export interface Connection {
  readonly kind: ConnectionKind;
  readonly summary: string;
}

export const NOT_YET_CHECKED: Connection = {
  kind: 'checking',
  summary: 'Checking the voice server…',
};
