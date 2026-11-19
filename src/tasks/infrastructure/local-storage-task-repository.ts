import { arrayOf, isBoolean, objectOf, parseJson } from '../../shared/checks.ts';
import type { TaskRepository } from '../application/ports.ts';
import { isTaskId, type Task } from '../domain/task.ts';
import { isTaskTitle } from '../domain/task-title.ts';

export const STORAGE_KEY = 'tasks.v1';

const isStoredTasks = arrayOf(objectOf({ id: isTaskId, title: isTaskTitle, done: isBoolean }));

export class LocalStorageTaskRepository implements TaskRepository {
  readonly #storage: Storage;

  constructor(storage: Storage) {
    this.#storage = storage;
  }

  load(): readonly Task[] {
    const stored = parseJson(this.#storage.getItem(STORAGE_KEY));
    return isStoredTasks(stored) ? stored : [];
  }

  save(tasks: readonly Task[]): void {
    this.#storage.setItem(STORAGE_KEY, JSON.stringify(tasks));
  }
}
