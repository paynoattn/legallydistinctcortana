export type MoodId =
  'neutral' | 'calm' | 'cheerful' | 'alert' | 'urgent' | 'angry' | 'mad' | 'whisper' | 'deadpan';

export interface Mood {
  readonly id: MoodId;
  readonly label: string;
  readonly hint: string;
}

const MOOD_ORDER: readonly MoodId[] = [
  'neutral',
  'calm',
  'cheerful',
  'alert',
  'urgent',
  'angry',
  'mad',
  'whisper',
  'deadpan',
];

const DETAILS: Readonly<Record<MoodId, Omit<Mood, 'id'>>> = {
  neutral: { label: 'Neutral', hint: 'Plain and even' },
  calm: { label: 'Calm', hint: 'Slower and softer' },
  cheerful: { label: 'Cheerful', hint: 'Bright and a little quicker' },
  alert: { label: 'Alert', hint: 'Crisp, to get attention' },
  urgent: { label: 'Urgent', hint: 'Fast and loud' },
  angry: { label: 'Angry', hint: 'Clipped and forceful' },
  mad: { label: 'Mad', hint: 'Angry, turned all the way up' },
  whisper: { label: 'Whisper', hint: 'Quiet and slow' },
  deadpan: { label: 'Deadpan', hint: 'Flat and unimpressed' },
};

export const MOODS: readonly Mood[] = MOOD_ORDER.map((id) => ({ id, ...DETAILS[id] }));

const SHORTCUT_KEY = /^[1-9]$/;
const DECIMAL = 10;

export function isMoodId(value: unknown): value is MoodId {
  return MOOD_ORDER.some((id) => id === value);
}

export function moodLabel(id: MoodId): string {
  return DETAILS[id].label;
}

export function moodForKey(key: string): Mood | undefined {
  return SHORTCUT_KEY.test(key) ? MOODS[Number.parseInt(key, DECIMAL) - 1] : undefined;
}
