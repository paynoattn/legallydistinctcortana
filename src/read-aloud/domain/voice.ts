export interface Voice {
  readonly name: string;
  readonly description: string;
  readonly languages: readonly string[];
  readonly speakers: readonly string[];
}

export interface VoiceChoice {
  readonly name: string;
  readonly speaker: string | null;
}

export function normalizeLanguage(code: string): string {
  const separator = code.search(/[-_]/);
  return separator < 0
    ? code.toLowerCase()
    : `${code.slice(0, separator).toLowerCase()}_${code.slice(separator + 1).toUpperCase()}`;
}

function speaksLanguage(voice: Voice, language: string): boolean {
  return voice.languages.some((code) => normalizeLanguage(code) === language);
}

export function languagesOf(voices: readonly Voice[]): readonly string[] {
  const codes = voices.flatMap((voice) => voice.languages.map(normalizeLanguage));
  return [...new Set(codes)].sort((left, right) => left.localeCompare(right));
}

export function voicesIn(voices: readonly Voice[], language: string): readonly Voice[] {
  return voices.filter((voice) => speaksLanguage(voice, language));
}

export function pickLanguage(languages: readonly string[], preferred: string): string | null {
  const wanted = normalizeLanguage(preferred);
  const cut = wanted.indexOf('_');
  const base = cut < 0 ? wanted : wanted.slice(0, cut);
  return (
    languages.find((code) => code === wanted) ??
    languages.find((code) => code.startsWith(`${base}_`) || code === base) ??
    languages[0] ??
    null
  );
}

export function languageOfChoice(
  voices: readonly Voice[],
  choice: VoiceChoice | null,
): string | null {
  const voice = voices.find((candidate) => candidate.name === choice?.name);
  const [code] = voice?.languages ?? [];
  return code === undefined ? null : normalizeLanguage(code);
}

export function choiceFor(voice: Voice | undefined): VoiceChoice | null {
  return voice === undefined ? null : { name: voice.name, speaker: voice.speakers[0] ?? null };
}

export function isOffered(voices: readonly Voice[], choice: VoiceChoice): boolean {
  const voice = voices.find((candidate) => candidate.name === choice.name);
  return (
    voice !== undefined && (choice.speaker === null || voice.speakers.includes(choice.speaker))
  );
}

export function voiceLabel(voice: Voice): string {
  return voice.description === '' ? voice.name : voice.description;
}

export function languageLabel(code: string): string {
  const names = new Intl.DisplayNames(['en'], { type: 'language', fallback: 'code' });
  return `${names.of(code.replace('_', '-')) ?? code} (${code})`;
}

export interface VoiceOptions {
  readonly languages: readonly string[];
  readonly voices: readonly Voice[];
  readonly speakers: readonly string[];
  readonly language: string;
  readonly name: string;
  readonly speaker: string;
}

function speakersOf(voices: readonly Voice[], choice: VoiceChoice | null): readonly string[] {
  return voices.find((voice) => voice.name === choice?.name)?.speakers ?? [];
}

function selectedValues(
  language: string | null,
  choice: VoiceChoice | null,
): Pick<VoiceOptions, 'language' | 'name' | 'speaker'> {
  return {
    language: language ?? '',
    name: choice?.name ?? '',
    speaker: choice?.speaker ?? '',
  };
}

export function voiceOptions(
  voices: readonly Voice[],
  language: string | null,
  choice: VoiceChoice | null,
): VoiceOptions {
  return {
    languages: languagesOf(voices),
    voices: language === null ? [] : voicesIn(voices, language),
    speakers: speakersOf(voices, choice),
    ...selectedValues(language, choice),
  };
}

export const DEFAULT_VOICE_LABEL = 'Default voice';

export function chosenVoiceLabel(voices: readonly Voice[], choice: VoiceChoice | null): string {
  if (choice === null) {
    return DEFAULT_VOICE_LABEL;
  }
  const voice = voices.find((candidate) => candidate.name === choice.name);
  return voice === undefined ? choice.name : voiceLabel(voice);
}
