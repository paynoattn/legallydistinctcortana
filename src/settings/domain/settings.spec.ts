import { describe, expect, it } from 'vitest';
import {
  type Draft,
  errorsFor,
  HOST_MESSAGE,
  parseDraft,
  PORT_MESSAGE,
  SAME_SHORTCUT_MESSAGE,
  SETTINGS_FIELDS,
  type Settings,
  SHORTCUT_LABELS,
  sameDraft,
  toDraft,
  validate,
} from './settings.ts';

const SETTINGS: Settings = {
  dictateShortcut: 'F9',
  readAloudShortcut: 'Ctrl+Alt+R',
  voiceHost: '127.0.0.1',
  sttPort: 10_300,
  ttsPort: 10_200,
  sttLanguage: 'auto',
  reduceMotion: false,
  showOverlay: true,
  startAtSignIn: false,
};

const DRAFT = toDraft(SETTINGS);

const CHANGED: Draft = {
  dictateShortcut: 'F8',
  readAloudShortcut: 'F7',
  voiceHost: 'h',
  sttPort: '1',
  ttsPort: '2',
  sttLanguage: 'en',
  reduceMotion: true,
  showOverlay: false,
  startAtSignIn: true,
};

function draftWith(change: Partial<Draft>): Draft {
  return { ...DRAFT, ...change };
}

describe('toDraft', () => {
  it('turns the ports into text and keeps the rest', () => {
    expect(DRAFT).toEqual({ ...SETTINGS, sttPort: '10300', ttsPort: '10200' });
  });
});

describe('validate', () => {
  it('accepts the current settings', () => {
    expect(validate(DRAFT)).toEqual({});
  });

  it.each(['1', '80', '65535', '00080'])('accepts the port %s', (port) => {
    expect(validate(draftWith({ sttPort: port, ttsPort: port }))).toEqual({});
  });

  it.each(['0', '65536', '', '8a', 'a8', '1e3', ' 80', '-1', '80.5'])(
    'refuses the port "%s"',
    (port) => {
      expect(validate(draftWith({ sttPort: port }))).toEqual({ sttPort: PORT_MESSAGE });
      expect(validate(draftWith({ ttsPort: port }))).toEqual({ ttsPort: PORT_MESSAGE });
    },
  );

  it.each(['', ' ', 'my host', ' voice.local', 'voice.local '])(
    'refuses the host "%s"',
    (voiceHost) => {
      expect(validate(draftWith({ voiceHost }))).toEqual({ voiceHost: HOST_MESSAGE });
    },
  );

  it('accepts a single character host', () => {
    expect(validate(draftWith({ voiceHost: 'h' }))).toEqual({});
  });

  it('refuses the same shortcut twice, on both fields', () => {
    expect(validate(draftWith({ readAloudShortcut: 'F9' }))).toEqual({
      dictateShortcut: SAME_SHORTCUT_MESSAGE,
      readAloudShortcut: SAME_SHORTCUT_MESSAGE,
    });
  });

  it('explains each problem in words', () => {
    expect([PORT_MESSAGE, HOST_MESSAGE, SAME_SHORTCUT_MESSAGE]).toEqual([
      'Use a whole number from 1 to 65535.',
      'Enter a host name or address, with no spaces.',
      'Both shortcuts are the same. Pick a different one.',
    ]);
  });
});

describe('parseDraft', () => {
  it('turns a valid draft back into settings', () => {
    expect(parseDraft(draftWith({ sttPort: '10301' }))).toEqual({
      kind: 'valid',
      settings: { ...SETTINGS, sttPort: 10_301 },
    });
  });

  it('reports every problem in an invalid draft', () => {
    expect(parseDraft(draftWith({ voiceHost: '', ttsPort: '0' }))).toEqual({
      kind: 'invalid',
      errors: { voiceHost: HOST_MESSAGE, ttsPort: PORT_MESSAGE },
    });
  });
});

describe('errorsFor', () => {
  it('lets the core error win over the local one for the same field', () => {
    expect(
      errorsFor(draftWith({ sttPort: '0' }), { sttPort: 'Port in use.', sttLanguage: 'Unknown.' }),
    ).toEqual({ sttPort: 'Port in use.', sttLanguage: 'Unknown.' });
  });

  it('keeps local errors the core did not mention', () => {
    expect(errorsFor(draftWith({ voiceHost: '' }), {})).toEqual({ voiceHost: HOST_MESSAGE });
  });
});

describe('sameDraft', () => {
  it('treats equal values as the same draft', () => {
    expect(sameDraft(DRAFT, toDraft({ ...SETTINGS }))).toBe(true);
  });

  it.each(SETTINGS_FIELDS)('notices a change to %s', (field) => {
    expect(sameDraft(DRAFT, { ...DRAFT, [field]: CHANGED[field] })).toBe(false);
  });

  it('lists every settings field once', () => {
    expect(SETTINGS_FIELDS).toHaveLength(Object.keys(SETTINGS).length);
    expect(new Set(SETTINGS_FIELDS)).toEqual(new Set(Object.keys(SETTINGS)));
  });
});

describe('SHORTCUT_LABELS', () => {
  it('names both shortcuts', () => {
    expect(SHORTCUT_LABELS).toEqual({
      dictateShortcut: 'Dictate (hold to talk)',
      readAloudShortcut: 'Read selection aloud',
    });
  });
});
