import { describe, expect, it } from 'vitest';
import { type Task, toTaskId } from '../domain/task.ts';
import { parseTaskTitle } from '../domain/task-title.ts';
import { LocalStorageTaskRepository, STORAGE_KEY } from './local-storage-task-repository.ts';

function task(title: string, done = false): Task {
  const parsed = parseTaskTitle(title);
  if (!parsed.ok) {
    throw new Error('fixture title is invalid');
  }
  return { id: toTaskId(`id-${title}`), title: parsed.title, done };
}

const repository = (): LocalStorageTaskRepository =>
  new LocalStorageTaskRepository(window.localStorage);

describe('LocalStorageTaskRepository', () => {
  it('loads nothing when nothing was saved', () => {
    expect(repository().load()).toEqual([]);
  });

  it('round-trips saved tasks under its storage key', () => {
    const tasks = [task('first'), task('second', true)];
    repository().save(tasks);
    expect(JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? '')).toEqual(tasks);
    expect(repository().load()).toEqual(tasks);
  });

  it.each([
    ['not JSON', '{'],
    ['not a list', '{"id":"a","title":"a","done":false}'],
    ['an empty id', '[{"id":"","title":"a","done":false}]'],
    ['an untrimmed title', '[{"id":"a","title":" a ","done":false}]'],
    ['a non-string title', '[{"id":"a","title":1,"done":false}]'],
    ['a non-string id', '[{"id":1,"title":"a","done":false}]'],
    ['a missing done flag', '[{"id":"a","title":"a"}]'],
  ])('loads nothing from %s', (_case, raw) => {
    window.localStorage.setItem(STORAGE_KEY, raw);
    expect(repository().load()).toEqual([]);
  });

  it('uses a versioned key', () => {
    expect(STORAGE_KEY).toBe('tasks.v1');
  });
});
