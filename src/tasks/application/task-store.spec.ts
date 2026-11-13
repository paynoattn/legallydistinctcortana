import { describe, expect, it, vi } from 'vitest';
import { type Task, toTaskId } from '../domain/task.ts';
import { MAX_TITLE_LENGTH } from '../domain/task-title.ts';
import type { IdGenerator, TaskRepository } from './ports.ts';
import { TaskStore } from './task-store.ts';

class MemoryRepository implements TaskRepository {
  saved: (readonly Task[])[] = [];
  readonly #initial: readonly Task[];

  constructor(initial: readonly Task[] = []) {
    this.#initial = initial;
  }

  load(): readonly Task[] {
    return this.#initial;
  }

  save(tasks: readonly Task[]): void {
    this.saved.push(tasks);
  }
}

class CountingIds implements IdGenerator {
  #count = 0;

  next(): ReturnType<IdGenerator['next']> {
    this.#count += 1;
    return toTaskId(`task-${String(this.#count)}`);
  }
}

function setup(initial: readonly Task[] = []): { store: TaskStore; repository: MemoryRepository } {
  const repository = new MemoryRepository(initial);
  return { store: new TaskStore({ repository, ids: new CountingIds() }), repository };
}

function addOrFail(store: TaskStore, title: string): Task {
  const result = store.add(title);
  if (result.kind !== 'added') {
    throw new Error(`expected "${title}" to be added`);
  }
  return result.task;
}

describe('TaskStore', () => {
  it('starts with the tasks the repository loads', () => {
    const { store: seed } = setup();
    const existing = addOrFail(seed, 'existing');
    expect(setup([existing]).store.getSnapshot()).toEqual([existing]);
  });

  it('adds an open task with a trimmed title and a fresh id', () => {
    const { store, repository } = setup();
    const result = store.add('  Buy milk  ');
    const expected = { id: 'task-1', title: 'Buy milk', done: false };
    expect(result).toEqual({ kind: 'added', task: expected });
    expect(store.getSnapshot()).toEqual([expected]);
    expect(repository.saved).toEqual([[expected]]);
  });

  it('keeps tasks in creation order', () => {
    const { store } = setup();
    addOrFail(store, 'first');
    addOrFail(store, 'second');
    expect(store.getSnapshot().map((task) => task.title)).toEqual(['first', 'second']);
  });

  it.each([
    ['   ', 'empty'],
    ['a'.repeat(MAX_TITLE_LENGTH + 1), 'too-long'],
  ])('rejects %o as %s without saving or notifying', (title, error) => {
    const { store, repository } = setup();
    const listener = vi.fn<() => void>();
    store.subscribe(listener);
    expect(store.add(title)).toEqual({ kind: 'rejected', error });
    expect(store.getSnapshot()).toEqual([]);
    expect(repository.saved).toEqual([]);
    expect(listener).not.toHaveBeenCalled();
  });

  it('completes a task, saves, and notifies', () => {
    const { store, repository } = setup();
    const task = addOrFail(store, 'finish me');
    const listener = vi.fn<() => void>();
    store.subscribe(listener);
    store.complete(task.id);
    expect(store.getSnapshot()).toEqual([{ ...task, done: true }]);
    expect(repository.saved.at(-1)).toEqual([{ ...task, done: true }]);
    expect(listener).toHaveBeenCalledOnce();
  });

  it('treats completing twice, or completing an unknown id, as a no-op', () => {
    const { store, repository } = setup();
    const task = addOrFail(store, 'finish me');
    store.complete(task.id);
    const snapshot = store.getSnapshot();
    const listener = vi.fn<() => void>();
    store.subscribe(listener);
    store.complete(task.id);
    store.complete(toTaskId('missing'));
    expect(store.getSnapshot()).toBe(snapshot);
    expect(repository.saved).toHaveLength(2);
    expect(listener).not.toHaveBeenCalled();
  });

  it('notifies every subscriber until it unsubscribes', () => {
    const { store } = setup();
    const kept = vi.fn<() => void>();
    const dropped = vi.fn<() => void>();
    store.subscribe(kept);
    const unsubscribe = store.subscribe(dropped);
    addOrFail(store, 'one');
    unsubscribe();
    addOrFail(store, 'two');
    expect(kept).toHaveBeenCalledTimes(2);
    expect(dropped).toHaveBeenCalledOnce();
  });

  it('returns the same snapshot until something changes', () => {
    const { store } = setup();
    const before = store.getSnapshot();
    expect(store.getSnapshot()).toBe(before);
    addOrFail(store, 'one');
    expect(store.getSnapshot()).not.toBe(before);
  });

  it('keeps the identity of unchanged tasks so rows can skip re-rendering', () => {
    const { store } = setup();
    const first = addOrFail(store, 'first');
    const second = addOrFail(store, 'second');
    store.complete(second.id);
    const [firstAfter, secondAfter] = store.getSnapshot();
    expect(firstAfter).toBe(first);
    expect(secondAfter).not.toBe(second);
  });
});
