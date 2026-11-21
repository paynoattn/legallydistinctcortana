import assert from 'node:assert/strict';
import type { Locator } from 'playwright';
import type { Place, Timing } from './parameter-types.ts';
import { emit, eventually, type ReaderSetup, type TasksWorld } from './world.ts';

export const MOOD_KEY = 'read-aloud.mood.v1';
export const VOICE_KEY = 'read-aloud.voice.v1';
export const LONG_SPEECH_MS = 5000;
export const LONG_PARAGRAPHS = 10;
export const LONG_SENTENCES_PER_PARAGRAPH = 4;
export const NEXT_FRAME_MS = 16;
export const FIRST_READING = 1;
export const SECOND_READING = 2;

const UNDER_WAY_SPEECH_MS = 2000;

export const VOICE_NAMES: Readonly<Record<string, string>> = {
  'amy (medium)': 'en_US-amy-medium',
  'libritts (high)': 'en_US-libritts-high',
  'thorsten (medium)': 'de_DE-thorsten-medium',
};

export const setups = new WeakMap<TasksWorld, ReaderSetup>();

export interface ReadRequest {
  readonly mood: string;
  readonly voice: string | null;
  readonly fromSentence: number;
}

interface Invocation {
  readonly command: string;
  readonly request: ReadRequest | null;
}

export function setupOf(world: TasksWorld): ReaderSetup {
  return setups.get(world) ?? {};
}

export function remember(world: TasksWorld, key: string, value: string): void {
  const setup = setupOf(world);
  setups.set(world, { ...setup, storage: { ...setup.storage, [key]: value } });
}

export function open(world: TasksWorld, text: string): Promise<void> {
  return world.openReadAloud(text, setupOf(world));
}

export function sentenceAt(world: TasksWorld, number: number): Locator {
  return world.page().locator(`.sentence[data-sentence="${String(number - 1)}"]`);
}

export function litWord(world: TasksWorld): Locator {
  return world.page().locator('.word.is-lit');
}

function numbered(count: number): string {
  return Array.from({ length: count }, (_, index) => `Here is sentence ${String(index + 1)}.`).join(
    ' ',
  );
}

export function longText(): string {
  return Array.from({ length: LONG_PARAGRAPHS }, (_, paragraph) =>
    Array.from(
      { length: LONG_SENTENCES_PER_PARAGRAPH },
      (_, sentence) =>
        `Paragraph ${String(paragraph + 1)} goes on with sentence ${String(sentence + 1)}, which is long enough to wrap across the line at least once.`,
    ).join(' '),
  ).join('\n');
}

export function startSentence(world: TasksWorld, number: number, timing: Timing): Promise<void> {
  return emit(world, 'read-aloud:sentence', { reading: 1, index: number - 1, ...timing });
}

export function passTime(world: TasksWorld, ms: number): Promise<void> {
  return world.page().clock.runFor(ms);
}

function readRequestOf(args: unknown): ReadRequest | null {
  const request: unknown =
    typeof args === 'object' && args !== null ? Reflect.get(args, 'request') : null;
  if (typeof request !== 'object' || request === null) {
    return null;
  }
  const voice: unknown = Reflect.get(request, 'voice');
  return {
    mood: String(Reflect.get(request, 'mood')),
    voice: typeof voice === 'string' ? voice : null,
    fromSentence: Number(Reflect.get(request, 'fromSentence')),
  };
}

export async function invocations(world: TasksWorld): Promise<readonly Invocation[]> {
  const log: unknown = await world
    .page()
    .evaluate((): unknown => Reflect.get(window, 'fakeIpcLog'));
  assert.ok(Array.isArray(log), 'the fake IPC log is missing');
  return log.map((entry: unknown) => ({
    command:
      typeof entry === 'object' && entry !== null ? String(Reflect.get(entry, 'command')) : '',
    request:
      typeof entry === 'object' && entry !== null
        ? readRequestOf(Reflect.get(entry, 'args'))
        : null,
  }));
}

export async function reads(world: TasksWorld): Promise<readonly ReadRequest[]> {
  return (await invocations(world)).flatMap((invocation) =>
    invocation.command === 'read_aloud' && invocation.request !== null ? [invocation.request] : [],
  );
}

export async function lastRead(world: TasksWorld, atLeast: number): Promise<ReadRequest> {
  const all = await reads(world);
  const last = all.at(-1);
  assert.ok(
    all.length >= atLeast && last !== undefined,
    `expected at least ${String(atLeast)} readings, got ${JSON.stringify(all)}`,
  );
  return last;
}

export async function expectReading(
  world: TasksWorld,
  expected: { readonly number: number; readonly again: boolean; readonly mood?: string },
): Promise<void> {
  await eventually(async () => {
    const read = await lastRead(world, expected.again ? SECOND_READING : FIRST_READING);
    assert.equal(read.fromSentence, expected.number - 1);
    if (expected.mood !== undefined) {
      assert.equal(read.mood, expected.mood.toLowerCase());
    }
  });
}

export async function statusSays(world: TasksWorld, text: string): Promise<void> {
  await eventually(async () => {
    const status = await world.page().getByRole('status').textContent();
    assert.ok(
      (status ?? '').includes(text),
      `expected the status to include "${text}", got "${String(status)}"`,
    );
  });
}

export async function underWay(world: TasksWorld, { number, count }: Place): Promise<void> {
  await open(world, numbered(count));
  await startSentence(world, number, { startsInMs: 0, speechMs: UNDER_WAY_SPEECH_MS });
  await statusSays(world, `Sentence ${String(number)} of ${String(count)}.`);
}

export async function noWordLit(world: TasksWorld): Promise<void> {
  await eventually(async () => {
    assert.equal(await litWord(world).count(), 0);
  });
}

export async function sentenceState(
  world: TasksWorld,
  number: number,
  state: string,
): Promise<void> {
  await eventually(async () => {
    assert.equal(await sentenceAt(world, number).getAttribute('data-state'), state);
  });
}
