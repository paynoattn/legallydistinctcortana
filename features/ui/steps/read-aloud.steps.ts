import assert from 'node:assert/strict';
import { Given, Then, When } from '@cucumber/cucumber';
import type { Locator } from 'playwright';
import { eventually, type TasksWorld } from './world.ts';

function moodItems(world: TasksWorld): Locator {
  return world.page().getByRole('menuitem');
}

function button(world: TasksWorld, name: string): Locator {
  return world.page().getByRole('button', { name, exact: true });
}

async function focusedMood(world: TasksWorld): Promise<string | null | undefined> {
  return world.page().evaluate(() => {
    return document.activeElement?.querySelector('.mood-label')?.textContent;
  });
}

Given('the text {string} is selected', async function (this: TasksWorld, text: string) {
  await this.openReadAloud(text);
});

When('I choose the {string} mood', async function (this: TasksWorld, label: string) {
  await moodItems(this)
    .filter({ has: this.page().locator('.mood-label', { hasText: label }) })
    .click();
});

When('I press {string}', async function (this: TasksWorld, key: string) {
  await this.page().keyboard.press(key);
});

When('I press the {word} button', async function (this: TasksWorld, name: string) {
  await button(this, name).click();
});

Then('I see the preview {string}', async function (this: TasksWorld, text: string) {
  await eventually(async () => {
    assert.equal(await this.page().getByRole('main').locator('blockquote').textContent(), text);
  });
});

Then('the moods are listed in order {string}', async function (this: TasksWorld, list: string) {
  await eventually(async () => {
    const labels = await moodItems(this).locator('.mood-label').allTextContents();
    assert.deepEqual(labels, list.split(', '));
  });
});

Then('the {string} mood is focused', async function (this: TasksWorld, label: string) {
  await eventually(async () => {
    assert.equal(await focusedMood(this), label);
  });
});

Then('the status says {string}', async function (this: TasksWorld, text: string) {
  await eventually(async () => {
    const status = await this.page().getByRole('status').textContent();
    assert.ok(
      (status ?? '').includes(text),
      `expected the status to include "${text}", got "${String(status)}"`,
    );
  });
});

Then('I see the alert {string}', async function (this: TasksWorld, message: string) {
  await eventually(async () => {
    assert.equal(await this.page().getByRole('alert').textContent(), message);
  });
});

Then('there is no Stop button', async function (this: TasksWorld) {
  await button(this, 'Stop').waitFor({ state: 'detached' });
});

Then('the Close button is focused', async function (this: TasksWorld) {
  await eventually(async () => {
    const focused = await this.page().evaluate(() => document.activeElement?.textContent);
    assert.equal(focused, 'Close');
  });
});

Then('there are no moods', async function (this: TasksWorld) {
  assert.equal(await moodItems(this).count(), 0);
});
