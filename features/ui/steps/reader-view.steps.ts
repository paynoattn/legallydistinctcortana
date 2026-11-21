import assert from 'node:assert/strict';
import { Then } from '@cucumber/cucumber';
import { litWord, noWordLit, sentenceState, statusSays } from './reader.ts';
import { eventually, type TasksWorld } from './world.ts';

const THIRDS = 3;
const FOLLOW_LINE = 1 / THIRDS;
const FOLLOW_TOLERANCE = 0.08;
const LIGHT_PAD_X = 5;
const MIST_DIM = 'rgb(163, 168, 208)';

async function heldBy(world: TasksWorld, paragraph: number): Promise<readonly number[]> {
  return world
    .page()
    .locator('.reader-text p')
    .nth(paragraph - 1)
    .locator('.sentence')
    .evaluateAll((spans) => spans.map((span) => Number(span.getAttribute('data-sentence')) + 1));
}

Then(/^I see (\d+) sentences?$/, async function (this: TasksWorld, count: string) {
  await eventually(async () => {
    assert.equal(await this.page().locator('.sentence').count(), Number(count));
  });
});

Then('I see {int} paragraphs', async function (this: TasksWorld, count: number) {
  await eventually(async () => {
    assert.equal(await this.page().locator('.reader-text p').count(), count);
  });
});

Then(
  'paragraph {int} holds sentence {int}',
  async function (this: TasksWorld, paragraph: number, sentence: number) {
    assert.deepEqual(await heldBy(this, paragraph), [sentence]);
  },
);

Then(
  'paragraph {int} holds sentences {pair}',
  async function (this: TasksWorld, paragraph: number, sentences: readonly number[]) {
    assert.deepEqual(await heldBy(this, paragraph), sentences);
  },
);

Then('the status says {string}', async function (this: TasksWorld, text: string) {
  await statusSays(this, text);
});

Then(
  'the status says {string} and no word is lit',
  async function (this: TasksWorld, text: string) {
    await statusSays(this, text);
    await noWordLit(this);
  },
);

Then('the mood button says {string}', async function (this: TasksWorld, label: string) {
  await eventually(async () => {
    const shown = await this.page()
      .getByRole('button', { name: /^Mood: / })
      .textContent();
    assert.equal(shown, label);
  });
});

Then('I see the heading {string}', async function (this: TasksWorld, name: string) {
  await this.page().getByRole('heading', { name, exact: true }).waitFor();
});

Then('I do not see {string}', async function (this: TasksWorld, text: string) {
  const shown = await this.page().locator('body').innerText();
  assert.ok(!shown.includes(text), `"${text}" is on screen`);
});

Then('the word {string} is lit', async function (this: TasksWorld, word: string) {
  await eventually(async () => {
    assert.deepEqual(await litWord(this).allTextContents(), [word]);
  });
});

Then('no word is lit', async function (this: TasksWorld) {
  await noWordLit(this);
});

Then(
  'sentence {int} is shown as read and sentence {int} as being read',
  async function (this: TasksWorld, read: number, now: number) {
    await sentenceState(this, read, 'read');
    await sentenceState(this, now, 'now');
  },
);

Then('sentence {int} is shown as being read', async function (this: TasksWorld, number: number) {
  await sentenceState(this, number, 'now');
});

Then(
  'sentences {int} and {int} are shown as ahead',
  async function (this: TasksWorld, first: number, second: number) {
    await sentenceState(this, first, 'ahead');
    await sentenceState(this, second, 'ahead');
  },
);

Then('the light moves to the next word without gliding', async function (this: TasksWorld) {
  await eventually(async () => {
    const word = await litWord(this).boundingBox();
    const light = await this.page().locator('.reader-light').boundingBox();
    assert.equal(await litWord(this).textContent(), 'emails');
    assert.ok(word !== null && light !== null, 'the lit word or the light is not rendered');
    assert.ok(Math.abs(light.x - (word.x - LIGHT_PAD_X)) < 1, 'the light is not on the lit word');
  });
  const durations = await this.page()
    .locator('.reader-light')
    .evaluate((light) => getComputedStyle(light).transitionDuration);
  assert.ok(/^0s(, 0s)*$/.test(durations), `the light glides: ${durations}`);
});

Then('the text is {int} px', async function (this: TasksWorld, size: number) {
  await eventually(async () => {
    const shown = await this.page()
      .locator('.reader-text')
      .evaluate((text) => getComputedStyle(text).fontSize);
    assert.equal(shown, `${String(size)}px`);
  });
});

Then('the lit line is back in view', async function (this: TasksWorld) {
  await eventually(async () => {
    const word = await litWord(this).boundingBox();
    const view = await this.page().locator('.reader-scroller').boundingBox();
    assert.ok(word !== null && view !== null, 'the lit word or the text is not rendered');
    assert.ok(word.y >= view.y && word.y + word.height <= view.y + view.height, 'out of view');
  });
});

Then('the lit line is about a third of the way down the text', async function (this: TasksWorld) {
  await eventually(async () => {
    const word = await litWord(this).boundingBox();
    const view = await this.page().locator('.reader-scroller').boundingBox();
    assert.ok(word !== null && view !== null, 'the lit word or the text is not rendered');
    const down = (word.y - view.y) / view.height;
    assert.ok(
      Math.abs(down - FOLLOW_LINE) < FOLLOW_TOLERANCE,
      `the lit line is at ${String(down)}`,
    );
  });
});

Then(
  'the progress shows {int} of {int} sentences done',
  async function (this: TasksWorld, done: number, count: number) {
    const progress = this.page().getByRole('progressbar');
    await eventually(async () => {
      assert.equal(await progress.getAttribute('aria-valuenow'), String(done));
      assert.equal(await progress.getAttribute('aria-valuemax'), String(count));
    });
  },
);

Then('the progress line is dimmed', async function (this: TasksWorld) {
  await eventually(async () => {
    const color = await this.page()
      .locator('.reader-progress i')
      .evaluate((line) => getComputedStyle(line).backgroundColor);
    assert.equal(color, MIST_DIM);
  });
});

Then('I see the alert {string}', async function (this: TasksWorld, message: string) {
  await eventually(async () => {
    assert.equal(await this.page().getByRole('alert').textContent(), message);
  });
});
