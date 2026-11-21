import assert from 'node:assert/strict';
import {
  After,
  AfterAll,
  Before,
  BeforeAll,
  setDefaultTimeout,
  setWorldConstructor,
  World,
} from '@cucumber/cucumber';
import { type Browser, chromium, type Page } from 'playwright';
import { createServer, type ViteDevServer } from 'vite';

const STEP_TIMEOUT_MS = 30_000;
const APP_READY_TIMEOUT_MS = 10_000;
const EVENTUALLY_TIMEOUT_MS = 5_000;
const POLL_INTERVAL_MS = 25;

setDefaultTimeout(STEP_TIMEOUT_MS);

let server: ViteDevServer | undefined;
let browser: Browser | undefined;
let baseUrl = '';

function startedBrowser(): Browser {
  if (browser === undefined) {
    throw new Error('the browser did not start');
  }
  return browser;
}

function recordBrowserErrors(page: Page, errors: string[]): void {
  page.on('pageerror', (error) => {
    errors.push(`uncaught: ${error.message}`);
  });
  page.on('console', (message) => {
    if (message.type() === 'error') {
      errors.push(`console.error: ${message.text()}`);
    }
  });
}

async function waitForApp(page: Page, errors: readonly string[], heading = 'Tasks'): Promise<void> {
  try {
    await page.getByRole('heading', { name: heading }).waitFor({ timeout: APP_READY_TIMEOUT_MS });
  } catch (error) {
    const reasons = errors.length > 0 ? errors.join('\n') : String(error);
    throw new Error(`the app did not render at ${page.url()}:\n${reasons}`, { cause: error });
  }
}

const CLOCK_START = new Date('2026-11-20T09:00:00Z');
const CLOCK_PAUSED = new Date('2026-11-20T10:00:00Z');

export interface ReaderSetup {
  readonly storage?: Readonly<Record<string, string>>;
  readonly reducedMotion?: boolean;
}

function recordReader(storage: Readonly<Record<string, string>>): void {
  if (window.sessionStorage.getItem('acceptance.seeded') === null) {
    window.sessionStorage.setItem('acceptance.seeded', 'yes');
    for (const [key, value] of Object.entries(storage)) {
      window.localStorage.setItem(key, value);
    }
  }
  const log: unknown[] = [];
  Reflect.set(window, 'fakeIpcLog', log);
  window.addEventListener('fake-ipc:invoked', (event) => {
    log.push(event instanceof CustomEvent ? event.detail : null);
  });
}

export class TasksWorld extends World {
  readonly errors: string[] = [];
  #page: Page | undefined;

  page(): Page {
    if (this.#page === undefined) {
      throw new Error('the browser page is not open yet');
    }
    return this.#page;
  }

  async open(): Promise<void> {
    const context = await startedBrowser().newContext();
    this.#page = await context.newPage();
    recordBrowserErrors(this.#page, this.errors);
    await this.#page.goto(baseUrl);
    await waitForApp(this.#page, this.errors);
  }

  async openReadAloud(selection: string, setup: ReaderSetup = {}): Promise<void> {
    const context = await startedBrowser().newContext({
      reducedMotion: setup.reducedMotion === true ? 'reduce' : 'no-preference',
    });
    await context.addInitScript(recordReader, setup.storage ?? {});
    this.#page = await context.newPage();
    recordBrowserErrors(this.#page, this.errors);
    await this.#page.clock.install({ time: CLOCK_START });
    const query = new URLSearchParams({ view: 'read-aloud', selection });
    await this.#page.goto(`${baseUrl}?${query.toString()}`);
    await waitForApp(this.#page, this.errors, 'Read aloud');
    await this.#page.clock.pauseAt(CLOCK_PAUSED);
  }

  async openView(view: string, ready: (page: Page) => Promise<void>): Promise<void> {
    const context = await startedBrowser().newContext();
    this.#page = await context.newPage();
    recordBrowserErrors(this.#page, this.errors);
    await this.#page.goto(`${baseUrl}?${new URLSearchParams({ view }).toString()}`);
    try {
      await ready(this.#page);
    } catch (error) {
      const reasons = this.errors.length > 0 ? this.errors.join('\n') : String(error);
      throw new Error(`the ${view} view did not render:\n${reasons}`, { cause: error });
    }
  }

  async reload(): Promise<void> {
    await this.page().reload();
    await waitForApp(this.page(), this.errors);
  }

  async close(): Promise<void> {
    await this.#page?.context().close();
  }
}

export async function emit(world: TasksWorld, event: string, payload: unknown): Promise<void> {
  await world.page().evaluate(
    ({ name, detail }) => {
      window.dispatchEvent(new CustomEvent(`fake-ipc:${name}`, { detail }));
    },
    { name: event, detail: payload },
  );
}

export async function eventually(assertion: () => Promise<void>): Promise<void> {
  const deadline = Date.now() + EVENTUALLY_TIMEOUT_MS;
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

setWorldConstructor(TasksWorld);

BeforeAll(async function () {
  server = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' });
  await server.listen();
  baseUrl = server.resolvedUrls?.local[0] ?? '';
  browser = await chromium.launch();
  const smoke = await browser.newPage();
  const errors: string[] = [];
  recordBrowserErrors(smoke, errors);
  try {
    await smoke.goto(baseUrl);
    await waitForApp(smoke, errors);
  } finally {
    await smoke.close();
  }
});

AfterAll(async function () {
  await browser?.close();
  await server?.close();
});

Before(
  { tags: 'not @read-aloud and not @overlay and not @settings' },
  async function (this: TasksWorld) {
    await this.open();
  },
);

After(async function (this: TasksWorld) {
  await this.close();
  assert.deepEqual(this.errors, [], 'the browser reported errors during this scenario');
});
