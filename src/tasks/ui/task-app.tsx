import { type ReactElement, useSyncExternalStore } from 'react';
import type { TaskStore } from '../application/task-store.ts';
import { NewTaskForm } from './new-task-form.tsx';
import { TaskList } from './task-list.tsx';

interface TaskAppProps {
  readonly store: TaskStore;
}

export function TaskApp({ store }: TaskAppProps): ReactElement {
  const tasks = useSyncExternalStore(store.subscribe, store.getSnapshot);
  return (
    <main>
      <h1>Tasks</h1>
      <NewTaskForm onAdd={store.add} />
      <TaskList tasks={tasks} onComplete={store.complete} />
    </main>
  );
}
