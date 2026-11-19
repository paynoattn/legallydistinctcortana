export interface Settings {
  readonly dictateShortcut: string;
  readonly readAloudShortcut: string;
  readonly voiceHost: string;
  readonly sttPort: number;
  readonly ttsPort: number;
  readonly sttLanguage: string;
  readonly reduceMotion: boolean;
  readonly showOverlay: boolean;
  readonly startAtSignIn: boolean;
}

export type SettingsField = keyof Settings;
export type ShortcutField = 'dictateShortcut' | 'readAloudShortcut';
type PortField = 'sttPort' | 'ttsPort';
export type FieldErrors = Readonly<Partial<Record<SettingsField, string>>>;
export type Draft = Readonly<Omit<Settings, PortField> & Record<PortField, string>>;

export type ParsedDraft =
  | { readonly kind: 'valid'; readonly settings: Settings }
  | { readonly kind: 'invalid'; readonly errors: FieldErrors };

export const SETTINGS_FIELDS: readonly SettingsField[] = [
  'dictateShortcut',
  'readAloudShortcut',
  'voiceHost',
  'sttPort',
  'ttsPort',
  'sttLanguage',
  'reduceMotion',
  'showOverlay',
  'startAtSignIn',
];

export const SHORTCUT_LABELS: Readonly<Record<ShortcutField, string>> = {
  dictateShortcut: 'Dictate (hold to talk)',
  readAloudShortcut: 'Read selection aloud',
};

export const PORT_MESSAGE = 'Use a whole number from 1 to 65535.';
export const HOST_MESSAGE = 'Enter a host name or address, with no spaces.';
export const SAME_SHORTCUT_MESSAGE = 'Both shortcuts are the same. Pick a different one.';

const MAX_PORT = 65_535;
const DIGITS = /^\d+$/;
const NO_SPACES = /^\S+$/;

export function toDraft(settings: Settings): Draft {
  return { ...settings, sttPort: String(settings.sttPort), ttsPort: String(settings.ttsPort) };
}

function isPort(text: string): boolean {
  const port = Number(text);
  return DIGITS.test(text) && port >= 1 && port <= MAX_PORT;
}

function portErrors(draft: Draft): FieldErrors {
  return {
    ...(isPort(draft.sttPort) ? {} : { sttPort: PORT_MESSAGE }),
    ...(isPort(draft.ttsPort) ? {} : { ttsPort: PORT_MESSAGE }),
  };
}

function shortcutErrors(draft: Draft): FieldErrors {
  return draft.dictateShortcut === draft.readAloudShortcut
    ? { dictateShortcut: SAME_SHORTCUT_MESSAGE, readAloudShortcut: SAME_SHORTCUT_MESSAGE }
    : {};
}

export function validate(draft: Draft): FieldErrors {
  return {
    ...shortcutErrors(draft),
    ...(NO_SPACES.test(draft.voiceHost) ? {} : { voiceHost: HOST_MESSAGE }),
    ...portErrors(draft),
  };
}

export function parseDraft(draft: Draft): ParsedDraft {
  const errors = validate(draft);
  return Object.keys(errors).length > 0
    ? { kind: 'invalid', errors }
    : {
        kind: 'valid',
        settings: { ...draft, sttPort: Number(draft.sttPort), ttsPort: Number(draft.ttsPort) },
      };
}

export function errorsFor(draft: Draft, coreErrors: FieldErrors): FieldErrors {
  return { ...validate(draft), ...coreErrors };
}

export function sameDraft(left: Draft, right: Draft): boolean {
  return SETTINGS_FIELDS.every((field) => left[field] === right[field]);
}
