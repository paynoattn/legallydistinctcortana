import type { Task, TaskId } from '../domain/task.ts';

export interface TaskRepository {
  load(): readonly Task[];
  save(tasks: readonly Task[]): void;
}

export interface IdGenerator {
  next(): TaskId;
}
