import assert from 'node:assert/strict';
import { Given, Then, When } from '@cucumber/cucumber';
import type { Place, Timing } from './parameter-types.ts';
import {
  expectReading,
  FIRST_READING,
  invocations,
  LONG_PARAGRAPHS,
  LONG_SENTENCES_PER_PARAGRAPH,
  LONG_SPEECH_MS,
  lastRead,
  litWord,
  longText,
  MOOD_KEY,
  NEXT_FRAME_MS,
  open,
  passTime,
  reads,
  remember,
  SECOND_READING,
  sentenceAt,
  setupOf,
  setups,
  startSentence,
  statusSays,
  underWay,
} from './reader.ts';
import { emit, eventually, type TasksWorld } from './world.ts';

const DEFAULT_UNDER_WAY: Place = { number: 1, count: 3 };
const STOP_AND_DISMISS = ['stop_reading', 'dismiss_read_aloud'];

Given('the last mood was {string}', function (this: TasksWorld, label: string) {
  remember(this, MOOD_KEY, label.toLowerCase());
});

Given('no mood was used before', function (this: TasksWorld) {
  setups.set(this, setupOf(this));
});

Given('the system prefers reduced motion', function (this: TasksWorld) {
  setups.set(this, { ...setupOf(this), reducedMotion: true });
});

Given('the text {string} is selected', async function (this: TasksWorld, text: string) {
  await open(this, text);
});

When('the text below is selected:', async function (this: TasksWorld, text: string) {
  await open(this, text);
});

When(
  'the text {string} is selected while the reader is open',
  async function (this: TasksWorld, text: string) {
    await this.page().evaluate((selected) => {
      window.dispatchEvent(new CustomEvent('fake-ipc:capture', { detail: selected }));
    }, text);
  },
);

Given(
  'a reading is under way at sentence {place}',
  async function (this: TasksWorld, place: Place) {
    await underWay(this, place);
  },
);

Given(
  'a reading is under way at sentence {place} in the {string} mood',
  async function (this: TasksWorld, place: Place, label: string) {
    remember(this, MOOD_KEY, label.toLowerCase());
    await underWay(this, place);
  },
);

Given('a reading is under way', async function (this: TasksWorld) {
  await underWay(this, DEFAULT_UNDER_WAY);
});

Given('a long reading is under way', async function (this: TasksWorld) {
  await open(this, longText());
  await startSentence(this, 1, { startsInMs: 0, speechMs: LONG_SPEECH_MS });
  await litWord(this).waitFor();
});

When(
  'sentence {int} starts now with {int} ms of speech',
  async function (this: TasksWorld, number: number, speechMs: number) {
    await startSentence(this, number, { startsInMs: 0, speechMs });
  },
);

When(
  'sentence {int} starts in {timing} of speech',
  async function (this: TasksWorld, number: number, timing: Timing) {
    await startSentence(this, number, timing);
  },
);

When('{int} ms pass', async function (this: TasksWorld, ms: number) {
  await passTime(this, ms);
  await passTime(this, NEXT_FRAME_MS);
});

Given(
  'sentence {int} has played for {int} ms',
  async function (this: TasksWorld, _number: number, ms: number) {
    await passTime(this, ms);
  },
);

When('a sentence below the window starts', async function (this: TasksWorld) {
  await startSentence(this, LONG_PARAGRAPHS * LONG_SENTENCES_PER_PARAGRAPH, {
    startsInMs: 0,
    speechMs: LONG_SPEECH_MS,
  });
});

When('the reading finishes', async function (this: TasksWorld) {
  await emit(this, 'read-aloud:status', { kind: 'finished' });
});

When('the voice fails with {string}', async function (this: TasksWorld, message: string) {
  await emit(this, 'read-aloud:status', { kind: 'failed', message });
});

When('I press {string}', async function (this: TasksWorld, key: string) {
  await this.page().keyboard.press(key);
});

When('I press {string} {int} times', async function (this: TasksWorld, key: string, times: number) {
  for (let pressed = 0; pressed < times; pressed += 1) {
    await this.page().keyboard.press(key);
  }
});

When('I click a word in sentence {int}', async function (this: TasksWorld, number: number) {
  await sentenceAt(this, number).locator('.word').first().click();
});

When(
  'I choose the {string} mood from the mood button',
  async function (this: TasksWorld, label: string) {
    await this.page()
      .getByRole('button', { name: /^Mood: / })
      .click();
    await this.page()
      .getByRole('menuitemradio')
      .filter({ has: this.page().locator('.mood-label', { hasText: label }) })
      .click();
  },
);

When('I scroll the text with the wheel', async function (this: TasksWorld) {
  await this.page().locator('.reader-scroller').hover();
  await this.page().mouse.wheel(0, LONG_SPEECH_MS);
});

Then(
  'reading starts from sentence {int} in the {string} mood',
  async function (this: TasksWorld, number: number, mood: string) {
    await expectReading(this, { number, again: false, mood });
  },
);

Then('reading starts again from sentence {int}', async function (this: TasksWorld, number: number) {
  await expectReading(this, { number, again: true });
});

Then(
  'reading starts again from sentence {int} in the same mood',
  async function (this: TasksWorld, number: number) {
    await expectReading(this, { number, again: true });
    const [first] = await reads(this);
    assert.equal((await lastRead(this, SECOND_READING)).mood, first?.mood);
  },
);

Then(
  'reading starts again from sentence {int} in the {string} mood',
  async function (this: TasksWorld, number: number, mood: string) {
    await expectReading(this, { number, again: true, mood });
  },
);

Then('no new reading starts', async function (this: TasksWorld) {
  assert.equal((await reads(this)).length, FIRST_READING);
});

Then('nothing is read', async function (this: TasksWorld) {
  assert.deepEqual(await reads(this), []);
});

Then(
  'the reading is stopped and the status says {string}',
  async function (this: TasksWorld, text: string) {
    await statusSays(this, text);
    assert.equal((await invocations(this)).at(-1)?.command, 'stop_reading');
  },
);

Then('the reading is stopped and the window is dismissed', async function (this: TasksWorld) {
  await eventually(async () => {
    const commands = (await invocations(this)).map((invocation) => invocation.command);
    assert.deepEqual(commands.slice(-STOP_AND_DISMISS.length), STOP_AND_DISMISS);
  });
});

Then('the window is dismissed', async function (this: TasksWorld) {
  await eventually(async () => {
    assert.equal((await invocations(this)).at(-1)?.command, 'dismiss_read_aloud');
  });
});

Then(
  '{string} is remembered for the next reading',
  async function (this: TasksWorld, label: string) {
    const stored = await this.page().evaluate(
      (name) => window.localStorage.getItem(name),
      MOOD_KEY,
    );
    assert.equal(stored, label.toLowerCase());
  },
);
