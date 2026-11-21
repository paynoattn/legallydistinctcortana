import assert from 'node:assert/strict';
import { type Browser, chromium, type Page } from 'playwright';
import {
  capture,
  eventually,
  expectReads,
  manual,
  openReader,
  selectionOf,
  step,
  voicesFailWith,
} from './reader/harness.ts';
import { readsAfter } from './reader/reader-page.ts';

const VOICE_KEY = 'read-aloud.voice.v1';
const TEXT = selectionOf([['The sun is warm.', 'The dog sat down.', 'The cat ran off.']]);
const VOICES_DOWN = "can't reach the voice server at voice-box:10200: connection refused";

const select = (page: Page, name: string) => page.getByRole('combobox', { name, exact: true });

async function expectVoiceButton(page: Page, label: string): Promise<void> {
  await eventually(async () => {
    assert.equal(await page.getByRole('button', { name: `Voice: ${label}` }).count(), 1);
  });
}

async function pick(page: Page, name: string, value: string): Promise<void> {
  await readsAfter(
    page,
    () =>
      select(page, name)
        .selectOption(value)
        .then(() => undefined),
    { fromSentence: 0 },
  );
}

async function picksAVoice(page: Page): Promise<void> {
  await expectReads(page, 1, { voice: null, fromSentence: 0 });
  await page.getByRole('button', { name: 'Voice: Default voice' }).click();
  assert.equal(await select(page, 'Language').inputValue(), 'en_US');
  assert.equal(await select(page, 'Voice').inputValue(), '');
  await pick(page, 'Language', 'de_DE');
  await expectReads(page, 2, { voice: 'de_DE-thorsten-medium', speaker: null, fromSentence: 0 });
  assert.equal(await select(page, 'Voice').inputValue(), 'de_DE-thorsten-medium');
  await expectVoiceButton(page, 'thorsten (medium)');
  await select(page, 'Voice').press('Escape');
  await select(page, 'Voice').waitFor({ state: 'detached' });
  await readsAfter(page, () => capture(page, TEXT), {
    voice: 'de_DE-thorsten-medium',
    fromSentence: 0,
  });
  await page.getByRole('button', { name: 'Voice: thorsten (medium)' }).click();
  assert.equal(await select(page, 'Language').inputValue(), 'de_DE');
  assert.equal(await select(page, 'Voice').inputValue(), 'de_DE-thorsten-medium');
}

async function picksASpeaker(page: Page): Promise<void> {
  await pick(page, 'Language', 'en_US');
  await expectReads(page, 4, { voice: 'en_US-amy-medium' });
  await pick(page, 'Voice', 'en_US-libritts-high');
  const speakers = await select(page, 'Speaker').locator('option').allTextContents();
  assert.deepEqual(speakers, ['Default speaker', 'p239', 'p240']);
  await pick(page, 'Speaker', 'p240');
  await expectReads(page, 6, { voice: 'en_US-libritts-high', speaker: 'p240' });
  await pick(page, 'Voice', '');
  await expectReads(page, 7, { voice: null, speaker: null });
  await expectVoiceButton(page, 'Default voice');
  assert.equal(await page.evaluate((key) => localStorage.getItem(key), VOICE_KEY), null);
}

async function retriesVoices(browser: Browser): Promise<void> {
  const page = await openReader(browser, { selection: TEXT, voicesFailure: VOICES_DOWN });
  await page.getByRole('button', { name: 'Voice: Default voice' }).click();
  const alert = page.getByRole('alert');
  await eventually(async () => {
    assert.equal(await alert.textContent(), `Voices are unavailable: ${VOICES_DOWN} Retry`);
  });
  await voicesFailWith(page, null);
  await alert.getByRole('button', { name: 'Retry' }).click();
  await select(page, 'Voice').waitFor();
  assert.deepEqual(await select(page, 'Voice').locator('option').allTextContents(), [
    'Server default',
    'amy (medium)',
    'libritts (high)',
  ]);
  await page.context().close();
}

const browser = await chromium.launch();

try {
  const page = await openReader(browser, { selection: TEXT });
  await step(1, async () => {
    await picksAVoice(page);
    await picksASpeaker(page);
  });
  await page.context().close();
  manual(2, 'restoring the clipboard after a capture is done by the desktop shell');
  manual(3, 'restoring every clipboard format is done by the desktop shell');
  await step(4, () => retriesVoices(browser));
} finally {
  await browser.close();
}
