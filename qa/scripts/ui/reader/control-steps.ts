import assert from 'node:assert/strict';
import type { Page } from 'playwright';
import {
  capture,
  commands,
  eventually,
  expectCommandsSince,
  expectReads,
  expectStatus,
  selectionOf,
  speak,
} from './harness.ts';
import {
  expectFocused,
  expectLit,
  expectTextSize,
  expectThirdDown,
  light,
  passTime,
  readsAfter,
  SENTENCE_MS,
  SENTENCES,
  SENTENCES_PER_PARAGRAPH,
  wordAt,
} from './reader-page.ts';

export const SHORT = selectionOf([['The sun is warm.', 'The dog sat down.']]);
const LEFT_TEXT_SIZE = 23;
const PAUSED_AT = 2 * SENTENCES_PER_PARAGRAPH + 1;
const LATER = PAUSED_AT + 2;

export async function choosesWhilePaused(page: Page): Promise<void> {
  const before = (await commands(page)).length;
  await page.keyboard.press('Space');
  await expectCommandsSince(page, before, ['stop_reading']);
  await readsAfter(page, () => page.keyboard.press('3'), {
    mood: 'cheerful',
    fromSentence: PAUSED_AT,
  });
  await page.keyboard.press('Space');
  await expectStatus(page, `Paused. Sentence ${String(PAUSED_AT + 1)} of ${String(SENTENCES)}.`);
  await readsAfter(page, () => wordAt(page, LATER, 3).click(), {
    mood: 'cheerful',
    fromSentence: LATER,
  });
}

async function resize(page: Page, key: string, sizes: readonly number[]): Promise<void> {
  for (const size of sizes) {
    await page.keyboard.press(key);
    await expectTextSize(page, size);
    await expectLit(page, LATER, 2);
  }
}

export async function sizesTheText(page: Page): Promise<void> {
  await speak(page, { index: LATER, startsInMs: 0, speechMs: SENTENCE_MS });
  await passTime(page, 600);
  await expectLit(page, LATER, 2);
  await expectTextSize(page, 25);
  await resize(page, 'Control+Equal', [28]);
  await resize(page, 'Control+Equal', [31, 34, 37, 40, 40, 40, 40, 40, 40, 40]);
  await resize(page, 'Control+Minus', [37, 34, 31, 28, 25, 22, 20, 20, 20, 20]);
  await resize(page, 'Control+NumpadAdd', [LEFT_TEXT_SIZE]);
  assert.equal(
    await light(page).evaluate((element) => element.classList.contains('is-off')),
    false,
  );
}

export async function stopsFollowingOnScroll(page: Page): Promise<void> {
  const follow = page.getByRole('button', { name: 'Follow the voice' });
  const scroller = page.locator('.reader-scroller');
  const box = await scroller.boundingBox();
  assert.ok(box !== null, 'the text is on screen');
  assert.equal(await follow.count(), 0);
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.wheel(0, -300);
  await follow.waitFor();
  await follow.click();
  await follow.waitFor({ state: 'detached' });
  await expectThirdDown(page);
  await page.mouse.move(box.x + box.width - 3, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width - 3, box.y + box.height / 2 + 60);
  await page.mouse.up();
  await follow.waitFor();
  await follow.click();
  await follow.waitFor({ state: 'detached' });
}

export async function keepsReadingWithoutFocus(page: Page): Promise<void> {
  const before = (await commands(page)).length;
  await page.evaluate(() => {
    document.activeElement?.dispatchEvent(new FocusEvent('focusout', { bubbles: true }));
    window.dispatchEvent(new FocusEvent('blur'));
  });
  await passTime(page, 1000);
  await expectStatus(page, `Sentence ${String(LATER + 1)} of ${String(SENTENCES)}.`);
  await expectLit(page, LATER, 6);
  assert.deepEqual((await commands(page)).slice(before), [], 'nothing stopped or dismissed');
}

export async function readsANewCapture(page: Page): Promise<void> {
  await readsAfter(page, () => capture(page, SHORT), { mood: 'cheerful', fromSentence: 0 });
  assert.deepEqual(await page.locator('.sentence').allTextContents(), [
    'The sun is warm.',
    'The dog sat down.',
  ]);
  await expectStatus(page, 'Sentence 1 of 2.');
  await expectFocused(page, 'Pause');
}

export async function escapeStopsAndCloses(page: Page): Promise<void> {
  const before = (await commands(page)).length;
  await page.keyboard.press('Escape');
  await expectCommandsSince(page, before, ['stop_reading', 'dismiss_read_aloud']);
}

export async function remembersSizeAndMood(page: Page): Promise<void> {
  const remembered = { mood: 'cheerful', voice: 'en_US-amy-medium', fromSentence: 0 };
  await readsAfter(page, () => capture(page, SHORT), remembered);
  await expectTextSize(page, LEFT_TEXT_SIZE);
  assert.equal(await page.getByRole('button', { name: 'Mood: Cheerful' }).count(), 1);
  await page.reload();
  await expectReads(page, 1, remembered);
  await expectTextSize(page, LEFT_TEXT_SIZE);
  assert.equal(await page.getByRole('button', { name: 'Mood: Cheerful' }).count(), 1);
  await eventually(async () => {
    assert.equal(await page.getByRole('button', { name: 'Voice: amy (medium)' }).count(), 1);
  });
  const before = (await commands(page)).length;
  await page.getByRole('button', { name: 'Close' }).click();
  await expectCommandsSince(page, before, ['stop_reading', 'dismiss_read_aloud']);
}
