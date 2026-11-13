import { completeTask, type Task, type TaskId } from '../domain/task.ts';
import { parseTaskTitle, type TitleError } from '../domain/task-title.ts';
import type { IdGenerator, TaskRepository } from './ports.ts';

type Listener = () => void;

export type AddResult =
  | { readonly kind: 'added'; readonly task: Task }
  | { readonly kind: 'rejected'; readonly error: TitleError };

export interface TaskStoreDeps {
  readonly repository: TaskRepository;
  readonly ids: IdGenerator;
}

export class TaskStore {
  readonly #listeners = new Set<Listener>();
  readonly #deps: TaskStoreDeps;
  #tasks: readonly Task[];

  constructor(deps: TaskStoreDeps) {
    this.#deps = deps;
    this.#tasks = deps.repository.load();
  }

  readonly subscribe = (listener: Listener): (() => void) => {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  };

  readonly getSnapshot = (): readonly Task[] => this.#tasks;

  readonly add = (rawTitle: string): AddResult => {
    const parsed = parseTaskTitle(rawTitle);
    if (!parsed.ok) {
      return { kind: 'rejected', error: parsed.error };
    }
    const task: Task = { id: this.#deps.ids.next(), title: parsed.title, done: false };
    this.#commit([...this.#tasks, task]);
    return { kind: 'added', task };
  };

  readonly complete = (id: TaskId): void => {
    const next = this.#tasks.map((task) => (task.id === id ? completeTask(task) : task));
    if (next.every((task, index) => task === this.#tasks[index])) {
      return;
    }
    this.#commit(next);
  };

  #commit(next: readonly Task[]): void {
    this.#tasks = next;
    this.#deps.repository.save(next);
    for (const listener of this.#listeners) {
      listener();
    }
  }
}
