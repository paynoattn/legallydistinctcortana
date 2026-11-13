import assert from 'node:assert/strict';
import { Given, Then, When } from '@cucumber/cucumber';
import type { Locator } from 'playwright';
import { eventually, type TasksWorld } from './world.ts';

const STORAGE_KEY = 'tasks.v1';

function titleField(world: TasksWorld): Locator {
  return world.page().getByRole('textbox', { name: 'Title' });
}

function checkbox(world: TasksWorld, title: string): Locator {
  return world.page().getByRole('checkbox', { name: title, exact: true });
}

async function addTask(world: TasksWorld, title: string): Promise<void> {
  await titleField(world).fill(title);
  await world.page().getByRole('button', { name: 'Add' }).click();
}

async function taskTitles(world: TasksWorld): Promise<string[]> {
  const list = world.page().getByRole('list', { name: 'Tasks' });
  return (await list.count()) === 0 ? [] : list.getByRole('listitem').allTextContents();
}

Given('I open the app', async function (this: TasksWorld) {
  await titleField(this).waitFor();
});

When('I reload the app', async function (this: TasksWorld) {
  await this.reload();
});

Given('the saved tasks are unreadable', async function (this: TasksWorld) {
  await this.page().evaluate(
    ([key]) => {
      window.localStorage.setItem(key, '{');
    },
    [STORAGE_KEY] as const,
  );
});

When(/^I add(?:ed)? a task titled "(.*)"$/, async function (this: TasksWorld, title: string) {
  await addTask(this, title);
});

When(
  'I add a task with a title of {int} characters',
  async function (this: TasksWorld, length: number) {
    await addTask(this, 'a'.repeat(length));
  },
);

When(/^I complete(?:d)? "(.*)"$/, async function (this: TasksWorld, title: string) {
  await checkbox(this, title).check();
});

Then('I see {string}', async function (this: TasksWorld, text: string) {
  await this.page().getByText(text, { exact: true }).waitFor();
});

Then('I see the error {string}', async function (this: TasksWorld, message: string) {
  await eventually(async () => {
    assert.equal(await this.page().getByRole('alert').textContent(), message);
  });
});

Then('I see no error', async function (this: TasksWorld) {
  await this.page().getByRole('alert').waitFor({ state: 'detached' });
});

Then('the title field is empty', async function (this: TasksWorld) {
  await eventually(async () => {
    assert.equal(await titleField(this).inputValue(), '');
  });
});

Then('the title field contains {string}', async function (this: TasksWorld, text: string) {
  assert.equal(await titleField(this).inputValue(), text);
});

Then('the tasks are {string}', async function (this: TasksWorld, titles: string) {
  await eventually(async () => {
    assert.deepEqual(await taskTitles(this), titles.split(', '));
  });
});

Then(/^the task is (added|rejected)$/, async function (this: TasksWorld, outcome: string) {
  await eventually(async () => {
    assert.equal((await taskTitles(this)).length, outcome === 'added' ? 1 : 0);
    assert.equal(await this.page().getByRole('alert').count(), outcome === 'added' ? 0 : 1);
  });
});

Then(/^"(.*)" is (open|done)$/, async function (this: TasksWorld, title: string, state: string) {
  await eventually(async () => {
    assert.equal(await checkbox(this, title).isChecked(), state === 'done');
  });
});

Then('{string} cannot be completed again', async function (this: TasksWorld, title: string) {
  assert.equal(await checkbox(this, title).isDisabled(), true);
});
