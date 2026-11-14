import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  choiceFor,
  isOffered,
  languageLabel,
  languageOfChoice,
  languagesOf,
  normalizeLanguage,
  pickLanguage,
  type Voice,
  voiceLabel,
  voiceOptions,
  voicesIn,
} from './voice.ts';

function voice(name: string, languages: string[], extra: Partial<Voice> = {}): Voice {
  return { name, description: '', languages, speakers: [], ...extra };
}

const AMY = voice('en_US-amy-medium', ['en_US'], { description: 'amy (medium)' });
const LIBRITTS = voice('en_US-libritts-high', ['en_US'], {
  description: 'libritts (high)',
  speakers: ['p239', 'p240'],
});
const THORSTEN = voice('de_DE-thorsten-medium', ['de_DE'], { description: 'thorsten (medium)' });
const VOICES = [THORSTEN, AMY, LIBRITTS];

describe('normalizeLanguage', () => {
  it('treats a leading separator as an empty language with a region', () => {
    expect(normalizeLanguage('-us')).toBe('_US');
  });

  it.each([
    ['en-us', 'en_US'],
    ['en_us', 'en_US'],
    ['EN_gb', 'en_GB'],
    ['de-DE', 'de_DE'],
    ['en', 'en'],
    ['EN', 'en'],
    ['', ''],
  ])('turns %j into %j', (code, expected) => {
    expect(normalizeLanguage(code)).toBe(expected);
  });
});

describe('languagesOf', () => {
  it('lists each normalized language once, sorted', () => {
    expect(languagesOf(VOICES)).toEqual(['de_DE', 'en_US']);
  });

  it('normalizes the codes a voice reports', () => {
    expect(languagesOf([voice('a', ['fr-fr', 'en-us']), voice('b', ['en_US'])])).toEqual([
      'en_US',
      'fr_FR',
    ]);
  });

  it('is empty without voices', () => {
    expect(languagesOf([])).toEqual([]);
  });
});

describe('voicesIn', () => {
  it('keeps the voices that speak the language, in order', () => {
    expect(voicesIn(VOICES, 'en_US')).toEqual([AMY, LIBRITTS]);
    expect(voicesIn(VOICES, 'de_DE')).toEqual([THORSTEN]);
  });

  it('matches a voice by any of its languages, ignoring how the code is written', () => {
    const both = voice('multi', ['de-de', 'en_US']);
    expect(voicesIn([both], 'en_US')).toEqual([both]);
    expect(voicesIn([both], 'de_DE')).toEqual([both]);
  });

  it('finds nothing for a language nobody speaks', () => {
    expect(voicesIn(VOICES, 'fr_FR')).toEqual([]);
  });
});

describe('pickLanguage', () => {
  it('matches by an empty base language when the preferred code starts with a separator', () => {
    expect(pickLanguage(['en_US', '_x'], '_us')).toBe('_x');
  });

  it('prefers the exact language', () => {
    expect(pickLanguage(['en_US', 'en_GB'], 'en_GB')).toBe('en_GB');
    expect(pickLanguage(['en_GB', 'en_US'], 'en-us')).toBe('en_US');
  });

  it('falls back to another region of the same language', () => {
    expect(pickLanguage(['de_DE', 'en_US'], 'en_GB')).toBe('en_US');
  });

  it('falls back to the bare language', () => {
    expect(pickLanguage(['de', 'en'], 'en_GB')).toBe('en');
  });

  it('matches a bare preferred language to any region', () => {
    expect(pickLanguage(['de_DE', 'en_US'], 'en')).toBe('en_US');
  });

  it('does not take a longer language code for the same language', () => {
    expect(pickLanguage(['eng_X', 'en_US'], 'en')).toBe('en_US');
    expect(pickLanguage(['eng', 'de_DE'], 'en')).toBe('eng');
  });

  it('falls back to the first language', () => {
    expect(pickLanguage(['de_DE', 'fr_FR'], 'en_US')).toBe('de_DE');
  });

  it('is null without languages', () => {
    expect(pickLanguage([], 'en_US')).toBeNull();
  });
});

describe('languageOfChoice', () => {
  it('is the first language of the chosen voice, normalized', () => {
    expect(languageOfChoice(VOICES, { name: THORSTEN.name, speaker: null })).toBe('de_DE');
    expect(languageOfChoice([voice('x', ['fr-ca', 'en_US'])], { name: 'x', speaker: null })).toBe(
      'fr_CA',
    );
  });

  it('is null without a choice', () => {
    expect(languageOfChoice(VOICES, null)).toBeNull();
  });

  it('is null for a voice nobody offers', () => {
    expect(languageOfChoice(VOICES, { name: 'gone', speaker: null })).toBeNull();
  });

  it('is null for a voice without languages', () => {
    expect(languageOfChoice([voice('x', [])], { name: 'x', speaker: null })).toBeNull();
  });
});

describe('choiceFor', () => {
  it('is null without a voice', () => {
    expect(choiceFor(undefined)).toBeNull();
  });

  it('has no speaker for a single-speaker voice', () => {
    expect(choiceFor(AMY)).toEqual({ name: AMY.name, speaker: null });
  });

  it('takes the first speaker of a multi-speaker voice', () => {
    expect(choiceFor(LIBRITTS)).toEqual({ name: LIBRITTS.name, speaker: 'p239' });
  });
});

describe('isOffered', () => {
  it('accepts an offered voice without a speaker', () => {
    expect(isOffered(VOICES, { name: AMY.name, speaker: null })).toBe(true);
    expect(isOffered(VOICES, { name: LIBRITTS.name, speaker: null })).toBe(true);
  });

  it('accepts an offered speaker', () => {
    expect(isOffered(VOICES, { name: LIBRITTS.name, speaker: 'p240' })).toBe(true);
  });

  it('refuses a speaker the voice does not have', () => {
    expect(isOffered(VOICES, { name: LIBRITTS.name, speaker: 'p999' })).toBe(false);
    expect(isOffered(VOICES, { name: AMY.name, speaker: 'p239' })).toBe(false);
  });

  it('refuses a voice nobody offers', () => {
    expect(isOffered(VOICES, { name: 'gone', speaker: null })).toBe(false);
    expect(isOffered([], { name: AMY.name, speaker: null })).toBe(false);
  });
});

describe('voiceLabel', () => {
  it('is the description', () => {
    expect(voiceLabel(AMY)).toBe('amy (medium)');
  });

  it('is the name when there is no description', () => {
    expect(voiceLabel(voice('plain', ['en_US']))).toBe('plain');
  });
});

describe('languageLabel', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('names languages in English whatever the system language is', () => {
    const names = vi.spyOn(Intl, 'DisplayNames');
    expect(languageLabel('de_DE')).toBe('German (Germany) (de_DE)');
    expect(names.mock.calls.map(([locales]) => locales)).toEqual([['en']]);
  });

  it.each([
    ['en_US', 'American English (en_US)'],
    ['de_DE', 'German (Germany) (de_DE)'],
    ['en', 'English (en)'],
    ['xx', 'xx (xx)'],
  ])('labels %j as %j', (code, label) => {
    expect(languageLabel(code)).toBe(label);
  });
});

describe('voiceOptions', () => {
  it('lists every language, the voices in the language and the chosen voice speakers', () => {
    expect(voiceOptions(VOICES, 'en_US', { name: LIBRITTS.name, speaker: 'p239' })).toEqual({
      languages: ['de_DE', 'en_US'],
      voices: [AMY, LIBRITTS],
      speakers: ['p239', 'p240'],
      language: 'en_US',
      name: LIBRITTS.name,
      speaker: 'p239',
    });
  });

  it('offers no voices without a language', () => {
    expect(voiceOptions(VOICES, null, null)).toEqual({
      languages: ['de_DE', 'en_US'],
      voices: [],
      speakers: [],
      language: '',
      name: '',
      speaker: '',
    });
  });

  it('selects no speaker for a chosen voice without one', () => {
    expect(voiceOptions(VOICES, 'en_US', { name: AMY.name, speaker: null }).speaker).toBe('');
  });

  it('offers no speakers without a choice, or for a single-speaker voice', () => {
    expect(voiceOptions(VOICES, 'en_US', null).speakers).toEqual([]);
    expect(voiceOptions(VOICES, 'en_US', { name: AMY.name, speaker: null }).speakers).toEqual([]);
  });

  it('offers no speakers for a voice nobody offers', () => {
    expect(voiceOptions(VOICES, 'en_US', { name: 'gone', speaker: 'p1' }).speakers).toEqual([]);
  });
});
