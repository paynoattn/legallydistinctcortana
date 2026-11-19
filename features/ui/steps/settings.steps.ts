import assert from 'node:assert/strict';
import { Given, Then, When } from '@cucumber/cucumber';
import type { Locator } from 'playwright';
import { eventually, type TasksWorld } from './world.ts';

function button(world: TasksWorld, name: string): Locator {
  return world.page().getByRole('button', { name, exact: true });
}

function field(world: TasksWorld, label: string): Locator {
  return world.page().getByRole('textbox', { name: label, exact: true });
}

function toggle(world: TasksWorld, label: string): Locator {
  return world.page().getByRole('switch', { name: label, exact: true });
}

async function errorOf(world: TasksWorld, control: Locator): Promise<string | null> {
  const id = await control.getAttribute('aria-describedby');
  return id === null ? null : world.page().locator(`#${id}`).textContent();
}

async function expectError(world: TasksWorld, control: Locator, message: string): Promise<void> {
  await eventually(async () => {
    assert.equal(await errorOf(world, control), message);
  });
}

Given('the settings window is open', async function (this: TasksWorld) {
  await this.openView('settings', (page) =>
    page.getByRole('button', { name: 'Save', exact: true }).waitFor(),
  );
});

When('I change the {string} shortcut', async function (this: TasksWorld, label: string) {
  await button(this, `Change ${label}`).click();
});

When('I click the {string} button', async function (this: TasksWorld, name: string) {
  await button(this, name).click();
});

When(
  'I set the {string} field to {string}',
  async function (this: TasksWorld, label: string, value: string) {
    await field(this, label).fill(value);
  },
);

When('I turn on {string}', async function (this: TasksWorld, label: string) {
  await toggle(this, label).check();
});

Then(
  'the {string} shortcut is {string}',
  async function (this: TasksWorld, label: string, chord: string) {
    await eventually(async () => {
      const shown = await this.page()
        .locator('.shortcut', { hasText: label })
        .locator('kbd.chord')
        .textContent();
      assert.equal(shown, chord);
    });
  },
);

Then('I am asked to {string}', async function (this: TasksWorld, prompt: string) {
  await eventually(async () => {
    const focused = await this.page().evaluate(() => document.activeElement?.textContent);
    assert.equal(focused, prompt);
  });
});

Then('the {string} button is focused', async function (this: TasksWorld, name: string) {
  await eventually(async () => {
    const focused = await this.page().evaluate(() =>
      document.activeElement?.getAttribute('aria-label'),
    );
    assert.equal(focused, name);
  });
});

Then(
  'the {string} field is {string}',
  async function (this: TasksWorld, label: string, value: string) {
    await eventually(async () => {
      assert.equal(await field(this, label).inputValue(), value);
    });
  },
);

Then(
  /^the "(.*)" switch is (on|off)$/,
  async function (this: TasksWorld, label: string, state: string) {
    await eventually(async () => {
      assert.equal(await toggle(this, label).isChecked(), state === 'on');
    });
  },
);

Then(
  'the {string} field shows the error {string}',
  async function (this: TasksWorld, label: string, message: string) {
    await expectError(this, field(this, label), message);
    assert.equal(await field(this, label).getAttribute('aria-invalid'), 'true');
  },
);

Then(
  'the {string} button shows the error {string}',
  async function (this: TasksWorld, name: string, message: string) {
    await expectError(this, button(this, name), message);
  },
);

Then('Save is disabled', async function (this: TasksWorld) {
  assert.equal(await button(this, 'Save').isDisabled(), true);
});

Then('the save status says {string}', async function (this: TasksWorld, text: string) {
  await eventually(async () => {
    assert.equal(await this.page().locator('.notice').textContent(), text);
  });
});

Then('the connection status says {string}', async function (this: TasksWorld, text: string) {
  await eventually(async () => {
    const status = await this.page().locator('.connection-status').textContent();
    assert.equal(status, text);
  });
});
