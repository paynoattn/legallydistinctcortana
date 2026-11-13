import type { ReactElement } from 'react';
import type { Task, TaskId } from '../domain/task.ts';

interface TaskListProps {
  readonly tasks: readonly Task[];
  readonly onComplete: (id: TaskId) => void;
}

interface TaskRowProps {
  readonly task: Task;
  readonly onComplete: (id: TaskId) => void;
}

function TaskRow({ task, onComplete }: TaskRowProps): ReactElement {
  return (
    <li>
      <label>
        <input
          type="checkbox"
          checked={task.done}
          disabled={task.done}
          onChange={() => {
            onComplete(task.id);
          }}
        />
        <span>{task.title}</span>
      </label>
    </li>
  );
}

export function TaskList({ tasks, onComplete }: TaskListProps): ReactElement {
  if (tasks.length === 0) {
    return <p>No tasks yet.</p>;
  }
  return (
    <ul aria-label="Tasks">
      {tasks.map((task) => (
        <TaskRow key={task.id} task={task} onComplete={onComplete} />
      ))}
    </ul>
  );
}
