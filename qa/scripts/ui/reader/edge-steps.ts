import assert from 'node:assert/strict';
import type { Browser, Page } from 'playwright';
import { SHORT } from './control-steps.ts';
import {
  commands,
  expectCommandsSince,
  expectReads,
  expectStatus,
  expectText,
  fail,
  finish,
  openReader,
  reads,
  selectionOf,
  speak,
} from './harness.ts';
import {
  article,
  COLORS,
  css,
  expectCss,
  expectFocused,
  expectLit,
  expectNothingLit,
  light,
  passTime,
  readsAfter,
  SENTENCE_MS,
  WORD_MS,
} from './reader-page.ts';

const NOTHING_SELECTED = 'Select some text first, then press the read-aloud shortcut.';
const UNREACHABLE = "can't reach the voice server at voice-box:10200: connection refused";

async function closesWith(page: Page, action: () => Promise<void>): Promise<void> {
  const before = (await commands(page)).length;
  await action();
  await expectCommandsSince(page, before, ['stop_reading', 'dismiss_read_aloud']);
}

export async function finishesAndReadsAgain(browser: Browser): Promise<void> {
  const page = await openReader(browser, { selection: SHORT });
  await expectReads(page, 1, { fromSentence: 0 });
  await speak(page, { index: 0, startsInMs: 0, speechMs: 1000 });
  await speak(page, { index: 1, startsInMs: 1000, speechMs: 1000 });
  await passTime(page, 1500);
  await expectLit(page, 1, 1);
  await passTime(page, 600);
  await finish(page);
  await expectStatus(page, 'Finished reading.');
  await expectNothingLit(page);
  const progress = page.getByRole('progressbar');
  assert.equal(await progress.getAttribute('aria-valuenow'), '2');
  await expectCss(progress.locator('i'), 'background-color', COLORS.mistDim);
  await readsAfter(page, () => page.keyboard.press('Space'), { fromSentence: 0 });
  await page.context().close();
}

export async function reducedMotionJumps(browser: Browser): Promise<void> {
  const page = await openReader(browser, {
    selection: selectionOf(article()),
    reducedMotion: true,
  });
  await expectReads(page, 1, { fromSentence: 0 });
  await speak(page, { index: 0, startsInMs: 0, speechMs: SENTENCE_MS });
  await passTime(page, 100);
  await expectLit(page, 0, 0);
  await passTime(page, WORD_MS);
  await expectLit(page, 0, 1);
  assert.equal(await css(light(page), 'transition-duration'), '0s', 'the light does not glide');
  assert.equal(await css(page.locator('.reader-scroller'), 'scroll-behavior'), 'auto');
  await page.context().close();
}

export async function nothingSelected(browser: Browser): Promise<void> {
  const page = await openReader(browser, {
    selection: { kind: 'failed', message: NOTHING_SELECTED },
  });
  await expectText(page.getByRole('alert'), NOTHING_SELECTED);
  await expectFocused(page, 'Close');
  await closesWith(page, () => page.getByRole('button', { name: 'Close' }).click());
  assert.deepEqual(await reads(page), [], 'nothing is read');
  await page.context().close();
}

export async function voiceServerDown(browser: Browser): Promise<void> {
  const page = await openReader(browser, { selection: SHORT });
  await expectReads(page, 1, { fromSentence: 0 });
  await fail(page, UNREACHABLE);
  await expectStatus(page, UNREACHABLE);
  await expectNothingLit(page);
  await closesWith(page, () => page.keyboard.press('Escape'));
  await page.context().close();
}

export async function screenReaderShape(browser: Browser): Promise<void> {
  const page = await openReader(browser, { selection: selectionOf(article()) });
  await expectReads(page, 1, { fromSentence: 0 });
  const text = page.locator('.reader-text');
  assert.equal(await page.getByRole('status').count(), 1, 'one status line');
  assert.equal(await text.getAttribute('aria-live'), 'off', 'lit words are not announced');
  assert.equal(await light(page).getAttribute('aria-hidden'), 'true');
  assert.equal(await text.locator('[role], [aria-live]:not([aria-live="off"])').count(), 0);
  assert.equal(await text.getByRole('paragraph').count(), 3, 'the text reads as paragraphs');
  await speak(page, { index: 0, startsInMs: 0, speechMs: SENTENCE_MS });
  await passTime(page, 100);
  await expectLit(page, 0, 0);
  await expectStatus(page, 'Sentence 1 of 18.');
  await page.context().close();
}
