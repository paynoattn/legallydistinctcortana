import assert from 'node:assert/strict';
import { type Browser, type Locator, type Page } from 'playwright';
import {
  type CoreSetup,
  installCommands,
  installControls,
  installTauriBridge,
  type Invocation,
} from './fake-core.ts';

const POLL_TIMEOUT_MS = 5_000;
const POLL_INTERVAL_MS = 25;
const CLOCK_START = new Date('2026-11-20T09:00:00Z');
const CLOCK_PAUSED = new Date('2026-11-20T10:00:00Z');

const WINDOW = { width: 860, height: 660 };

const VOICES = [
  { name: 'en_US-amy-medium', description: 'amy (medium)', languages: ['en_US'], speakers: [] },
  {
    name: 'en_US-libritts-high',
    description: 'libritts (high)',
    languages: ['en_US'],
    speakers: ['p239', 'p240'],
  },
  {
    name: 'de_DE-thorsten-medium',
    description: 'thorsten (medium)',
    languages: ['de_DE'],
    speakers: [],
  },
];

export interface ReadRequest {
  readonly mood: unknown;
  readonly voice: unknown;
  readonly speaker: unknown;
  readonly fromSentence: unknown;
}

export interface ReaderOptions {
  readonly selection: unknown;
  readonly voicesFailure?: string;
  readonly reducedMotion?: boolean;
}

const baseUrl = process.env['QA_BASE_URL'] ?? assert.fail('QA_BASE_URL is not set');

export async function eventually(assertion: () => Promise<void>): Promise<void> {
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

export async function step(n: number, run: () => Promise<void>): Promise<void> {
  await run();
  process.stdout.write(`step ${String(n)}: ok\n`);
}

export function manual(n: number, why: string): void {
  process.stdout.write(`step ${String(n)}: manual only (${why})\n`);
}

export function selectionOf(paragraphs: readonly (readonly string[])[]): unknown {
  const sentences = paragraphs.flatMap((paragraph) =>
    paragraph.map((text, index) => ({ text, startsParagraph: index === 0 })),
  );
  const text = paragraphs.map((paragraph) => paragraph.join(' ')).join('\n');
  return {
    kind: 'ready',
    preview: text.slice(0, 160),
    charCount: text.length,
    sentences,
  };
}

export async function openReader(browser: Browser, options: ReaderOptions): Promise<Page> {
  const context = await browser.newContext({
    viewport: WINDOW,
    locale: 'en-US',
    reducedMotion: options.reducedMotion === true ? 'reduce' : 'no-preference',
  });
  const setup: CoreSetup = {
    selection: options.selection,
    voices: VOICES,
    voicesFailure: options.voicesFailure ?? null,
  };
  await context.addInitScript(installTauriBridge);
  await context.addInitScript(installCommands, setup);
  await context.addInitScript(installControls);
  const page = await context.newPage();
  page.on('pageerror', (error) => {
    process.stderr.write(`page error: ${error.message}\n`);
  });
  await page.clock.install({ time: CLOCK_START });
  await page.goto(`${baseUrl}?view=read-aloud`);
  await page.getByRole('heading', { name: 'Read aloud' }).waitFor();
  await page.clock.pauseAt(CLOCK_PAUSED);
  return page;
}

async function invocations(page: Page): Promise<readonly Invocation[]> {
  return page.evaluate(() => window.qaBus?.log ?? []);
}

export async function commands(page: Page): Promise<readonly string[]> {
  return (await invocations(page))
    .map(({ command }) => command)
    .filter((command) => !command.startsWith('plugin:'));
}

function field(record: object, key: string): unknown {
  const value: unknown = Reflect.get(record, key);
  return value;
}

export async function reads(page: Page): Promise<readonly ReadRequest[]> {
  return (await invocations(page)).flatMap(({ command, args }) => {
    const request: unknown = args?.['request'];
    if (command !== 'read_aloud' || typeof request !== 'object' || request === null) {
      return [];
    }
    return [
      {
        mood: field(request, 'mood'),
        voice: field(request, 'voice'),
        speaker: field(request, 'speaker'),
        fromSentence: field(request, 'fromSentence'),
      },
    ];
  });
}

export async function expectReads(
  page: Page,
  count: number,
  last: Partial<ReadRequest>,
): Promise<void> {
  await eventually(async () => {
    const all = await reads(page);
    assert.equal(all.length, count, `read_aloud calls: ${JSON.stringify(all)}`);
    const latest = all.at(-1);
    assert.deepEqual({ ...latest, ...last }, latest, `last read_aloud call`);
  });
}

export async function expectCommandsSince(
  page: Page,
  before: number,
  expected: readonly string[],
): Promise<void> {
  await eventually(async () => {
    assert.deepEqual((await commands(page)).slice(before), expected);
  });
}

export async function expectText(locator: Locator, expected: string): Promise<void> {
  await eventually(async () => {
    assert.equal(await locator.textContent(), expected);
  });
}

export async function expectStatus(page: Page, expected: string): Promise<void> {
  await expectText(page.getByRole('status'), expected);
}

export async function speak(
  page: Page,
  sentence: { readonly index: number; readonly startsInMs: number; readonly speechMs: number },
): Promise<void> {
  await page.evaluate(({ index, startsInMs, speechMs }) => {
    window.qaCore?.sentence(index, startsInMs, speechMs);
  }, sentence);
}

export async function capture(page: Page, selection: unknown): Promise<void> {
  await page.evaluate((next) => {
    window.qaCore?.capture(next);
  }, selection);
}

export async function finish(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.qaCore?.finish();
  });
}

export async function fail(page: Page, message: string): Promise<void> {
  await page.evaluate((text) => {
    window.qaCore?.fail(text);
  }, message);
}

export async function voicesFailWith(page: Page, message: string | null): Promise<void> {
  await page.evaluate((text) => {
    window.qaCore?.voicesFailWith(text);
  }, message);
}
