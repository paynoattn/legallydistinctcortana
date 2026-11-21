import assert from 'node:assert/strict';
import type { Locator, Page } from 'playwright';
import { eventually, expectReads, reads, type ReadRequest } from './harness.ts';

const WORDS = [
  'the',
  'fox',
  'ran',
  'past',
  'old',
  'red',
  'barn',
  'and',
  'then',
  'sat',
  'down',
  'by',
  'a',
  'pond',
  'to',
  'rest',
  'in',
  'sun',
  'with',
  'dog',
  'cat',
  'big',
  'hill',
];

export const WORDS_PER_SENTENCE = 12;
export const WORD_MS = 250;
export const SENTENCE_MS = WORDS_PER_SENTENCE * WORD_MS;
export const PARAGRAPHS = 3;
export const SENTENCES_PER_PARAGRAPH = 6;
export const SENTENCES = PARAGRAPHS * SENTENCES_PER_PARAGRAPH;

export const COLORS = {
  dusk: 'rgb(31, 35, 71)',
  glow: 'rgb(255, 184, 28)',
  paper: 'rgb(255, 241, 199)',
  mistDim: 'rgb(163, 168, 208)',
  read: 'rgb(144, 149, 194)',
};

function sentence(n: number): string {
  const words = Array.from(
    { length: WORDS_PER_SENTENCE },
    (_, k) => WORDS[(n * 5 + k) % WORDS.length] ?? 'the',
  );
  const text = words.join(' ');
  return `${text.charAt(0).toUpperCase()}${text.slice(1)}.`;
}

export function article(): readonly (readonly string[])[] {
  return Array.from({ length: PARAGRAPHS }, (_, p) =>
    Array.from({ length: SENTENCES_PER_PARAGRAPH }, (_, s) =>
      sentence(p * SENTENCES_PER_PARAGRAPH + s),
    ),
  );
}

export function sentenceAt(page: Page, index: number): Locator {
  return page.locator(`.sentence[data-sentence="${String(index)}"]`);
}

export function wordAt(page: Page, index: number, position: number): Locator {
  return sentenceAt(page, index).locator('.word').nth(position);
}

export function litWord(page: Page): Locator {
  return page.locator('.word.is-lit');
}

export function light(page: Page): Locator {
  return page.locator('.reader-light');
}

export async function css(locator: Locator, property: string): Promise<string> {
  return locator.evaluate(
    (element, name) => getComputedStyle(element).getPropertyValue(name),
    property,
  );
}

export async function expectCss(locator: Locator, property: string, value: string): Promise<void> {
  await eventually(async () => {
    assert.equal(await css(locator, property), value, property);
  });
}

export async function expectLit(page: Page, index: number, position: number): Promise<void> {
  await eventually(async () => {
    assert.equal(await litWord(page).count(), 1, 'lit words');
    assert.equal(
      await litWord(page).evaluate((word) =>
        word.closest('.sentence')?.getAttribute('data-sentence'),
      ),
      String(index),
      'lit sentence',
    );
    assert.equal(
      await litWord(page).evaluate((word) =>
        [...(word.parentElement?.querySelectorAll('.word') ?? [])].indexOf(word),
      ),
      position,
      'lit word position',
    );
  });
}

export async function expectNothingLit(page: Page): Promise<void> {
  await eventually(async () => {
    assert.equal(await litWord(page).count(), 0, 'lit words');
  });
}

export async function expectThirdDown(page: Page): Promise<void> {
  await eventually(async () => {
    const place = await litWord(page).evaluate((word) => {
      const scroller = word.closest('.reader-scroller');
      const top = scroller?.getBoundingClientRect().top ?? 0;
      return {
        offset: word.getBoundingClientRect().top - top,
        third: (scroller?.clientHeight ?? 0) / 3,
      };
    });
    assert.ok(
      Math.abs(place.offset - place.third) <= 4,
      `lit line is ${String(place.offset)} px down, a third is ${String(place.third)} px`,
    );
  });
}

export async function expectTextSize(page: Page, size: number): Promise<void> {
  await expectCss(page.locator('.reader-text'), 'font-size', `${String(size)}px`);
}

export async function expectFocused(page: Page, label: string): Promise<void> {
  await eventually(async () => {
    assert.equal(
      await page.evaluate(() => document.activeElement?.getAttribute('aria-label')),
      label,
    );
  });
}

export async function readsAfter(
  page: Page,
  action: () => Promise<void>,
  expected: Partial<ReadRequest>,
): Promise<void> {
  const before = (await reads(page)).length;
  await action();
  await expectReads(page, before + 1, expected);
}

export async function passTime(page: Page, ms: number): Promise<void> {
  await page.clock.runFor(ms);
}
