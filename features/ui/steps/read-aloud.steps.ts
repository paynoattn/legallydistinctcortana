import assert from 'node:assert/strict';
import { Given, Then, When } from '@cucumber/cucumber';
import type { Locator } from 'playwright';
import {
  FIRST_READING,
  lastRead,
  remember,
  SECOND_READING,
  VOICE_KEY,
  VOICE_NAMES,
} from './reader.ts';
import { eventually, type TasksWorld } from './world.ts';

function picker(world: TasksWorld, name: string): Locator {
  return world.page().getByRole('combobox', { name });
}

async function pick(world: TasksWorld, name: string, label: string): Promise<void> {
  await picker(world, name).selectOption({ label });
}

async function shownIn(world: TasksWorld, name: string): Promise<string | null> {
  return picker(world, name).locator('option:checked').textContent();
}

async function expectVoice(
  world: TasksWorld,
  readings: number,
  expected: { readonly number: number; readonly label: string },
): Promise<void> {
  await eventually(async () => {
    const read = await lastRead(world, readings);
    assert.equal(read.fromSentence, expected.number - 1);
    assert.equal(read.voice, VOICE_NAMES[expected.label]);
  });
}

Given('the last voice was {string}', function (this: TasksWorld, label: string) {
  remember(this, VOICE_KEY, JSON.stringify({ name: VOICE_NAMES[label] ?? label, speaker: null }));
});

When('I open the voice picker', async function (this: TasksWorld) {
  await this.page()
    .getByRole('button', { name: /^Voice: / })
    .click();
});

Then(
  'reading starts from sentence {int} in the voice {string}',
  async function (this: TasksWorld, number: number, label: string) {
    await expectVoice(this, FIRST_READING, { number, label });
  },
);

Then(
  'reading starts again from sentence {int} in the voice {string}',
  async function (this: TasksWorld, number: number, label: string) {
    await expectVoice(this, SECOND_READING, { number, label });
  },
);

Then('the voice button says {string}', async function (this: TasksWorld, label: string) {
  await eventually(async () => {
    const shown = await this.page()
      .getByRole('button', { name: /^Voice: / })
      .textContent();
    assert.equal(shown, label);
  });
});

When('I pick the language {string}', async function (this: TasksWorld, label: string) {
  await pick(this, 'Language', label);
});

When('I pick the voice {string}', async function (this: TasksWorld, label: string) {
  await pick(this, 'Voice', label);
});

When('I pick the speaker {string}', async function (this: TasksWorld, label: string) {
  await pick(this, 'Speaker', label);
});

When('I reload the window', async function (this: TasksWorld) {
  await this.page().reload();
  await this.page().getByRole('heading', { name: 'Read aloud' }).waitFor();
});

Then('the language is {string}', async function (this: TasksWorld, label: string) {
  await eventually(async () => {
    assert.equal(await shownIn(this, 'Language'), label);
  });
});

Then('the voice is {string}', async function (this: TasksWorld, label: string) {
  await eventually(async () => {
    assert.equal(await shownIn(this, 'Voice'), label);
  });
});

Then('the speaker is {string}', async function (this: TasksWorld, label: string) {
  await eventually(async () => {
    assert.equal(await shownIn(this, 'Speaker'), label);
  });
});

Then('the voices are listed in order {string}', async function (this: TasksWorld, list: string) {
  await eventually(async () => {
    const labels = await picker(this, 'Voice').locator('option').allTextContents();
    assert.deepEqual(labels, list.split(', '));
  });
});

Then('there is no speaker choice', async function (this: TasksWorld) {
  assert.equal(await picker(this, 'Speaker').count(), 0);
});
