import { describe, expect, it } from 'vitest';
import type { Connection } from '../domain/connection.ts';
import { FAKE_DEFAULTS, FakeSettingsGateway, LANGUAGE_MESSAGE } from './fake-settings-gateway.ts';

describe('FakeSettingsGateway', () => {
  it('starts with the defaults', async () => {
    const gateway = new FakeSettingsGateway();
    await expect(gateway.load()).resolves.toEqual(FAKE_DEFAULTS);
    await expect(gateway.defaults()).resolves.toEqual(FAKE_DEFAULTS);
    expect(FAKE_DEFAULTS).toEqual({
      dictateShortcut: 'F9',
      readAloudShortcut: 'Ctrl+Alt+R',
      voiceHost: '127.0.0.1',
      sttPort: 10_300,
      ttsPort: 10_200,
      sttLanguage: 'auto',
      reduceMotion: false,
      showOverlay: true,
      startAtSignIn: false,
    });
  });

  it('saves, lower-casing the host like a core would normalize it', async () => {
    const gateway = new FakeSettingsGateway();
    const settings = { ...FAKE_DEFAULTS, voiceHost: 'Voice.Local', sttLanguage: 'en' };
    const saved = { ...settings, voiceHost: 'voice.local' };
    await expect(gateway.save(settings)).resolves.toEqual({ kind: 'saved', settings: saved });
    await expect(gateway.load()).resolves.toEqual(saved);
  });

  it.each(['auto', 'en', 'deu'])('accepts the language %s', async (sttLanguage) => {
    const result = await new FakeSettingsGateway().save({ ...FAKE_DEFAULTS, sttLanguage });
    expect(result.kind).toBe('saved');
  });

  it.each(['English', 'e', 'EN', 'engl', 'xauto', 'autox'])(
    'rejects the language %s like a core would',
    async (sttLanguage) => {
      const gateway = new FakeSettingsGateway();
      await expect(gateway.save({ ...FAKE_DEFAULTS, sttLanguage })).resolves.toEqual({
        kind: 'rejected',
        errors: { sttLanguage: LANGUAGE_MESSAGE },
      });
      await expect(gateway.load()).resolves.toEqual(FAKE_DEFAULTS);
    },
  );

  it('reports a connection to the saved host and announces each check', async () => {
    const gateway = new FakeSettingsGateway();
    const seen: Connection[] = [];
    await gateway.save({ ...FAKE_DEFAULTS, voiceHost: 'box' });
    const connected = {
      kind: 'connected',
      summary: 'Speech recognition and voice are reachable at box.',
    };
    await expect(gateway.connectionStatus()).resolves.toEqual(connected);
    await expect(gateway.checkConnection()).resolves.toEqual(connected);
    gateway.onConnection((connection) => seen.push(connection));
    await gateway.checkConnection();
    expect(seen).toEqual([{ kind: 'checking', summary: 'Checking the voice server.' }]);
  });

  it('suspends and resumes shortcuts without doing anything', async () => {
    const gateway = new FakeSettingsGateway();
    await expect(gateway.suspendShortcuts()).resolves.toBeUndefined();
    await expect(gateway.resumeShortcuts()).resolves.toBeUndefined();
  });
});
