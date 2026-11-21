import assert from 'node:assert/strict';
import type { Page } from 'playwright';
import {
  commands,
  eventually,
  expectCommandsSince,
  expectReads,
  expectStatus,
  speak,
} from './harness.ts';
import {
  COLORS,
  css,
  expectCss,
  expectFocused,
  expectLit,
  expectThirdDown,
  light,
  litWord,
  passTime,
  PARAGRAPHS,
  readsAfter,
  SENTENCE_MS,
  sentenceAt,
  SENTENCES,
  SENTENCES_PER_PARAGRAPH,
  WORD_MS,
  WORDS_PER_SENTENCE,
  wordAt,
} from './reader-page.ts';

const NEUTRAL_FIRST = { mood: 'neutral', voice: null, speaker: null, fromSentence: 0 };

export async function opensAndReads(page: Page): Promise<void> {
  await expectReads(page, 1, NEUTRAL_FIRST);
  assert.equal(await page.getByRole('heading', { name: 'Read aloud' }).count(), 1);
  assert.equal(await page.getByRole('button', { name: 'Close' }).count(), 1);
  assert.equal(await page.getByText('Reading from').count(), 0);
  const paragraphs = page.locator('.reader-text p');
  assert.equal(await paragraphs.count(), PARAGRAPHS);
  for (let p = 0; p < PARAGRAPHS; p += 1) {
    assert.equal(await paragraphs.nth(p).locator('.sentence').count(), SENTENCES_PER_PARAGRAPH);
  }
  assert.equal(await page.getByRole('button', { name: 'Mood: Neutral' }).count(), 1);
  assert.equal(await page.getByRole('button', { name: 'Voice: Default voice' }).count(), 1);
  await expectStatus(page, `Sentence 1 of ${String(SENTENCES)}.`);
  const hint = await page.locator('.reader-hint').evaluate((element) => ({
    textWidth: element.scrollWidth,
    room: element.clientWidth,
    below:
      element.getBoundingClientRect().top >=
      (element.parentElement?.querySelector('.reader-row')?.getBoundingClientRect().bottom ?? 0),
  }));
  assert.ok(hint.below, 'the keyboard hint is under the controls');
  assert.ok(
    hint.textWidth <= hint.room,
    `the keyboard hint needs ${String(hint.textWidth)} px on its line but has ${String(hint.room)} px, so it is cut off`,
  );
  await expectFocused(page, 'Pause');
}

function lineTop(word: Element): number {
  return word instanceof HTMLElement ? word.offsetTop : Number.NaN;
}

async function expectJumpsOnlyOnNewLines(page: Page, index: number): Promise<void> {
  let lastTop = await litWord(page).evaluate(lineTop);
  let newLines = 0;
  for (let position = 1; position < WORDS_PER_SENTENCE; position += 1) {
    await passTime(page, WORD_MS);
    await expectLit(page, index, position);
    const top = await litWord(page).evaluate(lineTop);
    const jumped = await light(page).evaluate((element) => element.hasAttribute('data-jump'));
    assert.equal(jumped, top !== lastTop, `light jump at word ${String(position + 1)}`);
    newLines += top === lastTop ? 0 : 1;
    lastTop = top;
  }
  assert.ok(newLines > 0, `sentence ${String(index + 1)} should wrap onto a new line`);
}

export async function lightsTheSpokenWord(page: Page): Promise<void> {
  await speak(page, { index: 0, startsInMs: 0, speechMs: SENTENCE_MS });
  await passTime(page, 100);
  await expectLit(page, 0, 0);
  await passTime(page, 500);
  await expectLit(page, 0, 2);
  assert.equal(await css(litWord(page), 'color'), COLORS.dusk);
  assert.equal(await css(light(page), 'background-color'), COLORS.glow);
  assert.match(await css(light(page), 'transition-property'), /transform/);
  await expectCss(sentenceAt(page, 0), 'color', COLORS.paper);
  await expectCss(sentenceAt(page, 1), 'color', COLORS.mistDim);
  await speak(page, { index: 1, startsInMs: SENTENCE_MS - 600, speechMs: SENTENCE_MS });
  await passTime(page, SENTENCE_MS - 600 + 100);
  await expectLit(page, 1, 0);
  await expectCss(sentenceAt(page, 0), 'color', COLORS.read);
  await expectCss(sentenceAt(page, 1), 'color', COLORS.paper);
  await expectCss(sentenceAt(page, 2), 'color', COLORS.mistDim);
  assert.equal(await page.getByRole('progressbar').getAttribute('aria-valuenow'), '1');
  await expectJumpsOnlyOnNewLines(page, 1);
}

export async function pausesAndResumes(page: Page): Promise<void> {
  const before = (await commands(page)).length;
  const lit = await litWord(page).textContent();
  await page.keyboard.press('Space');
  await expectCommandsSince(page, before, ['stop_reading']);
  await expectStatus(page, `Paused. Sentence 2 of ${String(SENTENCES)}.`);
  await passTime(page, 1000);
  assert.equal(await litWord(page).textContent(), lit, 'the light stops while paused');
  await readsAfter(page, () => page.keyboard.press('Space'), { mood: 'neutral', fromSentence: 1 });
  await expectStatus(page, `Sentence 2 of ${String(SENTENCES)}.`);
}

async function movesWith(
  page: Page,
  next: () => Promise<void>,
  previous: () => Promise<void>,
): Promise<void> {
  await readsAfter(page, next, { fromSentence: 2 });
  await readsAfter(page, previous, { fromSentence: 1 });
  await speak(page, { index: 1, startsInMs: 0, speechMs: SENTENCE_MS });
  await passTime(page, 3000);
  await readsAfter(page, previous, { fromSentence: 1 });
}

export async function movesBySentence(page: Page): Promise<void> {
  await movesWith(
    page,
    () => page.keyboard.press('ArrowRight'),
    () => page.keyboard.press('ArrowLeft'),
  );
  await movesWith(
    page,
    () => page.getByRole('button', { name: 'Next sentence' }).click(),
    () => page.getByRole('button', { name: 'Previous sentence' }).click(),
  );
}

export async function clickReadsFromSentence(page: Page): Promise<void> {
  const target = 2 * SENTENCES_PER_PARAGRAPH + 1;
  await readsAfter(page, () => wordAt(page, target, 4).click(), {
    mood: 'neutral',
    fromSentence: target,
  });
  await expectStatus(page, `Sentence ${String(target + 1)} of ${String(SENTENCES)}.`);
  await speak(page, { index: target, startsInMs: 0, speechMs: SENTENCE_MS });
  await passTime(page, 600);
  await expectLit(page, target, 2);
  await expectThirdDown(page);
}

export async function changesMood(page: Page): Promise<void> {
  const from = 2 * SENTENCES_PER_PARAGRAPH + 1;
  await readsAfter(page, () => page.keyboard.press('2'), { mood: 'calm', fromSentence: from });
  await page.getByRole('button', { name: 'Mood: Calm' }).click();
  await readsAfter(page, () => page.getByRole('menuitemradio', { name: /Whisper/ }).click(), {
    mood: 'whisper',
    fromSentence: from,
  });
  await eventually(async () => {
    assert.equal(await page.getByRole('button', { name: 'Mood: Whisper' }).count(), 1);
  });
}

export async function changesVoice(page: Page): Promise<void> {
  const from = 2 * SENTENCES_PER_PARAGRAPH + 1;
  await page.getByRole('button', { name: 'Voice: Default voice' }).click();
  const voice = page.getByRole('combobox', { name: 'Voice' });
  await readsAfter(page, () => voice.selectOption('en_US-amy-medium').then(() => undefined), {
    mood: 'whisper',
    voice: 'en_US-amy-medium',
    fromSentence: from,
  });
  await eventually(async () => {
    assert.equal(await page.getByRole('button', { name: 'Voice: amy (medium)' }).count(), 1);
  });
  const before = (await commands(page)).length;
  await voice.press('Escape');
  await voice.waitFor({ state: 'detached' });
  assert.deepEqual((await commands(page)).slice(before), [], 'Esc in the picker only closes it');
}
