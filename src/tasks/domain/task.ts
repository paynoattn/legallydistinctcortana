import type { TaskTitle } from './task-title.ts';

declare const brand: unique symbol;

export type TaskId = string & { readonly [brand]: 'TaskId' };

export interface Task {
  readonly id: TaskId;
  readonly title: TaskTitle;
  readonly done: boolean;
}

export class InvalidTaskIdError extends Error {
  constructor() {
    super('A task id must be a non-empty string.');
    this.name = 'InvalidTaskIdError';
  }
}

export function isTaskId(value: unknown): value is TaskId {
  return typeof value === 'string' && value.length > 0;
}

export function toTaskId(value: string): TaskId {
  if (isTaskId(value)) {
    return value;
  }
  throw new InvalidTaskIdError();
}

export function completeTask(task: Task): Task {
  return task.done ? task : { ...task, done: true };
}
