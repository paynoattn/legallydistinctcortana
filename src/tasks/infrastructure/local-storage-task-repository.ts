import { z } from 'zod/mini';
import type { TaskRepository } from '../application/ports.ts';
import { isTaskId, type Task, type TaskId } from '../domain/task.ts';
import { isTaskTitle, type TaskTitle } from '../domain/task-title.ts';

export const STORAGE_KEY = 'tasks.v1';

function parseJson(raw: string): unknown {
  try {
    const value: unknown = JSON.parse(raw);
    return value;
  } catch {
    return undefined;
  }
}

const storedTasks = z.catch(
  z.pipe(
    z.pipe(z.string(), z.transform(parseJson)),
    z.array(
      z.object({
        id: z.custom<TaskId>(isTaskId),
        title: z.custom<TaskTitle>(isTaskTitle),
        done: z.boolean(),
      }),
    ),
  ),
  [],
);

export class LocalStorageTaskRepository implements TaskRepository {
  readonly #storage: Storage;

  constructor(storage: Storage) {
    this.#storage = storage;
  }

  load(): readonly Task[] {
    return storedTasks.parse(this.#storage.getItem(STORAGE_KEY));
  }

  save(tasks: readonly Task[]): void {
    this.#storage.setItem(STORAGE_KEY, JSON.stringify(tasks));
  }
}
