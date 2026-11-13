import assert from 'node:assert/strict';
import { chromium, type Locator, type Page } from 'playwright';

const MAX_TITLE_LENGTH = 200;
const BULK_TASKS = 100;
const POLL_TIMEOUT_MS = 5_000;
const POLL_INTERVAL_MS = 25;

const baseUrl = process.env['QA_BASE_URL'] ?? assert.fail('QA_BASE_URL is not set');

declare global {
  interface Window {
    longFrames?: number[];
  }
}

async function eventually(assertion: () => Promise<void>): Promise<void> {
  const deadline = Date.now() + POLL_TIMEOUT_MS;
  for (;;) {
    try {
      await assertion();
      return;
    } catch (error) {
      if (Date.now() > deadline) {
        throw error;
      }
      await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
    }
  }
}

async function step(n: number, run: () => Promise<void>): Promise<void> {
  await run();
  process.stdout.write(`step ${String(n)}: ok\n`);
}

const titleField = (page: Page): Locator => page.getByRole('textbox', { name: 'Title' });
const alert = (page: Page): Locator => page.getByRole('alert');
const rows = (page: Page): Locator =>
  page.getByRole('list', { name: 'Tasks' }).getByRole('listitem');
const box = (page: Page, title: string): Locator =>
  page.getByRole('checkbox', { name: title, exact: true });

async function add(page: Page, title: string): Promise<void> {
  await titleField(page).fill(title);
  await page.getByRole('button', { name: 'Add' }).click();
}

async function expectTitles(page: Page, titles: string[]): Promise<void> {
  await eventually(async () => {
    assert.deepEqual(await rows(page).allTextContents(), titles);
  });
}

async function expectAlert(page: Page, message: string): Promise<void> {
  await eventually(async () => {
    assert.equal(await alert(page).textContent(), message);
  });
}

async function watchLongFrames(page: Page): Promise<void> {
  await page.evaluate(() => {
    const frames: number[] = [];
    window.longFrames = frames;
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        frames.push(Math.round(entry.duration));
      }
    }).observe({ type: 'long-animation-frame' });
  });
}

async function longFrames(page: Page): Promise<number[]> {
  return page.evaluate(() => window.longFrames ?? []);
}

const browser = await chromium.launch();
const page = await browser.newPage();
const long = 'a'.repeat(MAX_TITLE_LENGTH);

try {
  await step(1, async () => {
    await page.goto(baseUrl);
    await page.getByRole('heading', { name: 'Tasks' }).waitFor();
    await page.getByText('No tasks yet.', { exact: true }).waitFor();
    assert.equal(await titleField(page).inputValue(), '');
  });

  await step(2, async () => {
    await titleField(page).fill('  QA: buy milk  ');
    await titleField(page).press('Enter');
    await expectTitles(page, ['QA: buy milk']);
    assert.equal(await box(page, 'QA: buy milk').isChecked(), false);
    assert.equal(await titleField(page).inputValue(), '');
  });

  await step(3, async () => {
    await add(page, 'QA: second');
    await expectTitles(page, ['QA: buy milk', 'QA: second']);
  });

  await step(4, async () => {
    await add(page, '');
    await expectAlert(page, 'Enter a title.');
    assert.equal(await rows(page).count(), 2);
  });

  await step(5, async () => {
    await add(page, `${long}a`);
    await expectAlert(page, 'Keep the title to 200 characters or fewer.');
    assert.equal(await rows(page).count(), 2);
  });

  await step(6, async () => {
    await add(page, long);
    await alert(page).waitFor({ state: 'detached' });
    await expectTitles(page, ['QA: buy milk', 'QA: second', long]);
  });

  await step(7, async () => {
    await box(page, 'QA: buy milk').check();
    assert.equal(await box(page, 'QA: buy milk').isChecked(), true);
    assert.equal(await box(page, 'QA: buy milk').isDisabled(), true);
  });

  await step(8, async () => {
    await page.reload();
    await expectTitles(page, ['QA: buy milk', 'QA: second', long]);
    assert.equal(await box(page, 'QA: buy milk').isChecked(), true);
    assert.equal(await box(page, 'QA: second').isChecked(), false);
  });

  await step(9, async () => {
    await watchLongFrames(page);
    for (let i = 1; i <= BULK_TASKS; i += 1) {
      await add(page, `QA: bulk ${String(i)}`);
    }
    await eventually(async () => {
      assert.equal(await rows(page).count(), BULK_TASKS + 3);
    });
    assert.deepEqual(await longFrames(page), [], 'long animation frames (ms) while adding tasks');
  });
} finally {
  await browser.close();
}
