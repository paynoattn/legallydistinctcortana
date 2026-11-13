import { describe, expect, it } from 'vitest';
import { CryptoIdGenerator } from './crypto-id-generator.ts';

describe('CryptoIdGenerator', () => {
  it('returns a different UUID each time', () => {
    const ids = new CryptoIdGenerator();
    const first = ids.next();
    expect(first).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
    expect(ids.next()).not.toBe(first);
  });
});
