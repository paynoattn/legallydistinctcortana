import assert from 'node:assert/strict';
import { Given, Then, When } from '@cucumber/cucumber';
import type { Locator, Page } from 'playwright';
import { emit, eventually, type TasksWorld } from './world.ts';

const LOUD_GROWTH = 1.3;
const notedCoreWidths = new WeakMap<TasksWorld, number>();
const invocations = new WeakMap<TasksWorld, Promise<string>>();

function mark(world: TasksWorld): Locator {
  return world.page().locator('.lp');
}

function sendState(world: TasksWorld, kind: string, reduceMotion: boolean): Promise<void> {
  return emit(world, 'overlay:state', { state: { kind }, reduceMotion });
}

function sendError(world: TasksWorld, reason: string, fix: string | null): Promise<void> {
  return emit(world, 'overlay:state', {
    state: { kind: 'error', reason, fix },
    reduceMotion: false,
  });
}

async function coreWidth(world: TasksWorld): Promise<number> {
  const box = await world.page().locator('.lp-core').boundingBox();
  assert.ok(box !== null, 'the core is not rendered');
  return box.width;
}

function nextInvocation(page: Page): Promise<string> {
  return page.evaluate(
    () =>
      new Promise<string>((resolve) => {
        window.addEventListener(
          'fake-ipc:invoked',
          (event) => {
            resolve(event instanceof CustomEvent ? String(event.detail) : '');
          },
          { once: true },
        );
      }),
  );
}

Given('the indicator window is open', async function (this: TasksWorld) {
  await this.openView('overlay', (page) => page.locator('.lp').waitFor({ state: 'attached' }));
});

When('the core sends the {string} state', async function (this: TasksWorld, kind: string) {
  await sendState(this, kind, false);
});

When(
  'the core sends the {string} state with reduced motion',
  async function (this: TasksWorld, kind: string) {
    await sendState(this, kind, true);
  },
);

When('the core sends the level {float}', async function (this: TasksWorld, level: number) {
  await emit(this, 'overlay:level', { level });
});

When(
  'the core reports the error {string} with the fix {string}',
  async function (this: TasksWorld, reason: string, fix: string) {
    await sendError(this, reason, fix);
  },
);

When(
  'the core reports the error {string} with no fix',
  async function (this: TasksWorld, reason: string) {
    await sendError(this, reason, null);
  },
);

When('I note the size of the core', async function (this: TasksWorld) {
  await eventually(async () => {
    assert.equal(await mark(this).getAttribute('data-state'), 'listening');
  });
  notedCoreWidths.set(this, await coreWidth(this));
});

When('I click {string}', async function (this: TasksWorld, name: string) {
  invocations.set(
    this,
    nextInvocation(this.page()).catch(() => ''),
  );
  await this.page().getByRole('button', { name, exact: true }).click();
});

Then('the indicator is hidden', async function (this: TasksWorld) {
  await eventually(async () => {
    assert.equal(await mark(this).getAttribute('data-state'), 'hidden');
    assert.equal(await mark(this).evaluate((element) => getComputedStyle(element).opacity), '0');
  });
});

Then('the indicator is {string}', async function (this: TasksWorld, kind: string) {
  await eventually(async () => {
    assert.equal(await mark(this).getAttribute('data-state'), kind);
  });
});

Then('the check badge is shown', async function (this: TasksWorld) {
  await eventually(async () => {
    const opacity = await this.page()
      .locator('.lp-check')
      .evaluate((element) => getComputedStyle(element).opacity);
    assert.equal(opacity, '1');
  });
});

Then('the core is larger than noted', async function (this: TasksWorld) {
  const noted = notedCoreWidths.get(this) ?? Number.POSITIVE_INFINITY;
  await eventually(async () => {
    const width = await coreWidth(this);
    assert.ok(
      width > noted * LOUD_GROWTH,
      `expected the core to grow from ${String(noted)}, got ${String(width)}`,
    );
  });
});

Then('the indicator shows the reason {string}', async function (this: TasksWorld, reason: string) {
  await this.page().getByRole('button', { name: reason, exact: true }).waitFor();
  assert.equal(await mark(this).getAttribute('data-state'), 'error');
});

Then('it offers the fix {string}', async function (this: TasksWorld, label: string) {
  await this.page().getByRole('button', { name: label, exact: true }).waitFor();
});

Then('it offers no fix', async function (this: TasksWorld) {
  assert.equal(await this.page().locator('.err button').count(), 1);
});

Then('the core is asked to run {string}', async function (this: TasksWorld, command: string) {
  assert.equal(await invocations.get(this), command);
});

interface Motion {
  readonly transformed: (string | null)[];
  readonly animations: string[];
}

function motionIn(page: Page): Promise<Motion> {
  return page.evaluate(() => ({
    transformed: [...document.querySelectorAll('.overlay, .overlay *')]
      .filter((element) => getComputedStyle(element).transform !== 'none')
      .map((element) => element.getAttribute('class')),
    animations: document
      .getAnimations()
      .filter((animation) => animation instanceof CSSAnimation)
      .map((animation) => animation.animationName),
  }));
}

Then('nothing in the indicator moves', async function (this: TasksWorld) {
  await eventually(async () => {
    assert.deepEqual(await motionIn(this.page()), { transformed: [], animations: [] });
  });
});
