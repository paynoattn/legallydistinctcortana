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

  async openReadAloud(selection: string): Promise<void> {
    const context = await startedBrowser().newContext();
    this.#page = await context.newPage();
    recordBrowserErrors(this.#page, this.errors);
    const query = new URLSearchParams({ view: 'read-aloud', selection });
    await this.#page.goto(`${baseUrl}?${query.toString()}`);
    await waitForApp(this.#page, this.errors, 'Read aloud');
  }

  async reload(): Promise<void> {
    await this.page().reload();
    await waitForApp(this.page(), this.errors);
  }

  async close(): Promise<void> {
    await this.#page?.context().close();
  }
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

Before({ tags: 'not @read-aloud' }, async function (this: TasksWorld) {
  await this.open();
});

After(async function (this: TasksWorld) {
  await this.close();
  assert.deepEqual(this.errors, [], 'the browser reported errors during this scenario');
});
