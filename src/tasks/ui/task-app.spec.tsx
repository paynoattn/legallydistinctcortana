import { fireEvent, render, screen } from '@testing-library/react';
import { userEvent } from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { TaskStore } from '../application/task-store.ts';
import { CryptoIdGenerator } from '../infrastructure/crypto-id-generator.ts';
import { LocalStorageTaskRepository } from '../infrastructure/local-storage-task-repository.ts';
import { TaskApp } from './task-app.tsx';

function startApp(): ReturnType<typeof userEvent.setup> {
  const store = new TaskStore({
    repository: new LocalStorageTaskRepository(window.localStorage),
    ids: new CryptoIdGenerator(),
  });
  render(<TaskApp store={store} />);
  return userEvent.setup();
}

const titleInput = (): HTMLElement => screen.getByRole('textbox', { name: 'Title' });

async function addTask(user: ReturnType<typeof userEvent.setup>, title: string): Promise<void> {
  await user.clear(titleInput());
  if (title !== '') {
    await user.type(titleInput(), title);
  }
  await user.click(screen.getByRole('button', { name: 'Add' }));
}

describe('TaskApp', () => {
  it('shows an empty state before any task exists', () => {
    startApp();
    expect(screen.getByRole('heading', { name: 'Tasks' })).toBeTruthy();
    expect(screen.getByText('No tasks yet.')).toBeTruthy();
    expect(screen.queryByRole('list')).toBeNull();
    expect(titleInput()).toHaveProperty('value', '');
  });

  it('handles the submit itself instead of letting the browser navigate', () => {
    startApp();
    expect(fireEvent.submit(screen.getByRole('form', { name: 'New task' }))).toBe(false);
  });

  it('adds a trimmed task, lists it open, and clears the input', async () => {
    const user = startApp();
    await addTask(user, '  Buy milk  ');
    const checkbox = screen.getByRole('checkbox', { name: 'Buy milk' });
    expect(checkbox).toHaveProperty('checked', false);
    expect(checkbox).toHaveProperty('disabled', false);
    expect(titleInput()).toHaveProperty('value', '');
    expect(screen.queryByText('No tasks yet.')).toBeNull();
  });

  it('lists tasks in creation order', async () => {
    const user = startApp();
    await addTask(user, 'first');
    await addTask(user, 'second');
    const list = screen.getByRole('list', { name: 'Tasks' });
    expect(list.textContent).toBe('firstsecond');
  });

  it.each([
    ['', 'Enter a title.'],
    ['a'.repeat(201), 'Keep the title to 200 characters or fewer.'],
  ])('rejects %o, explains why, and keeps the input', async (title, message) => {
    const user = startApp();
    await addTask(user, title);
    expect(screen.getByRole('alert').textContent).toBe(message);
    expect(titleInput()).toHaveProperty('value', title);
    expect(titleInput().getAttribute('aria-invalid')).toBe('true');
    expect(titleInput().getAttribute('aria-describedby')).toBe(screen.getByRole('alert').id);
    expect(screen.getByText('No tasks yet.')).toBeTruthy();
  });

  it('clears the error once a valid task is added', async () => {
    const user = startApp();
    await addTask(user, '   ');
    await addTask(user, 'valid');
    expect(screen.queryByRole('alert')).toBeNull();
    expect(titleInput().getAttribute('aria-invalid')).toBe('false');
    expect(titleInput().hasAttribute('aria-describedby')).toBe(false);
  });

  it('completes a task once and keeps it done', async () => {
    const user = startApp();
    await addTask(user, 'finish me');
    await user.click(screen.getByRole('checkbox', { name: 'finish me' }));
    const checkbox = screen.getByRole('checkbox', { name: 'finish me' });
    expect(checkbox).toHaveProperty('checked', true);
    expect(checkbox).toHaveProperty('disabled', true);
  });
});
