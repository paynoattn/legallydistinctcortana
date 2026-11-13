import type { IdGenerator } from '../application/ports.ts';
import { type TaskId, toTaskId } from '../domain/task.ts';

export class CryptoIdGenerator implements IdGenerator {
  next(): TaskId {
    return toTaskId(crypto.randomUUID());
  }
}
