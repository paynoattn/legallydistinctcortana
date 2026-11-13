import { type ReactElement, type SubmitEvent, useState } from 'react';
import type { AddResult } from '../application/task-store.ts';
import { MAX_TITLE_LENGTH, type TitleError } from '../domain/task-title.ts';

const TITLE_ERRORS: Record<TitleError, string> = {
  empty: 'Enter a title.',
  'too-long': `Keep the title to ${String(MAX_TITLE_LENGTH)} characters or fewer.`,
};

interface NewTaskFormProps {
  readonly onAdd: (title: string) => AddResult;
}

export function NewTaskForm({ onAdd }: NewTaskFormProps): ReactElement {
  const [title, setTitle] = useState('');
  const [error, setError] = useState<TitleError | undefined>(undefined);

  function handleSubmit(event: SubmitEvent<HTMLFormElement>): void {
    event.preventDefault();
    const result = onAdd(title);
    if (result.kind === 'rejected') {
      setError(result.error);
      return;
    }
    setTitle('');
    setError(undefined);
  }

  return (
    <form aria-label="New task" onSubmit={handleSubmit}>
      <label htmlFor="new-task-title">Title</label>
      <input
        id="new-task-title"
        name="title"
        autoComplete="off"
        value={title}
        aria-invalid={error !== undefined}
        aria-describedby={error === undefined ? undefined : 'new-task-error'}
        onChange={(event) => {
          setTitle(event.target.value);
        }}
      />
      <button type="submit">Add</button>
      {error !== undefined && (
        <p id="new-task-error" role="alert">
          {TITLE_ERRORS[error]}
        </p>
      )}
    </form>
  );
}
