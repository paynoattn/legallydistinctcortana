import { describe, expect, it } from 'vitest';
import { completeTask, InvalidTaskIdError, isTaskId, type Task, toTaskId } from './task.ts';
import { parseTaskTitle } from './task-title.ts';

function openTask(): Task {
  const parsed = parseTaskTitle('Write tests');
  if (!parsed.ok) {
    throw new Error('fixture title is invalid');
  }
  return { id: toTaskId('task-1'), title: parsed.title, done: false };
}

describe('task ids', () => {
  it('accepts any non-empty string', () => {
    expect(isTaskId('x')).toBe(true);
    expect(toTaskId('x')).toBe('x');
  });

  it.each([1, null, undefined, {}])('rejects the non-string %o', (value) => {
    expect(isTaskId(value)).toBe(false);
  });

  it('rejects the empty string', () => {
    expect(isTaskId('')).toBe(false);
    expect(() => toTaskId('')).toThrow(
      expect.objectContaining({
        name: 'InvalidTaskIdError',
        message: 'A task id must be a non-empty string.',
      }),
    );
    expect(() => toTaskId('')).toThrow(InvalidTaskIdError);
  });
});

describe('completeTask', () => {
  it('returns a done copy and leaves the original open', () => {
    const task = openTask();
    expect(completeTask(task)).toEqual({ ...task, done: true });
    expect(task.done).toBe(false);
  });

  it('returns the same task when it is already done', () => {
    const done = completeTask(openTask());
    expect(completeTask(done)).toBe(done);
  });
});
