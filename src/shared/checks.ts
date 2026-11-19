export type Check<T> = (value: unknown) => value is T;

type Checked<C> = C extends Check<infer T> ? T : never;

type Shape = Readonly<Record<string, Check<unknown>>>;

type ShapeOf<S extends Shape> = { readonly [K in keyof S]: Checked<S[K]> };

export function isString(value: unknown): value is string {
  return typeof value === 'string';
}

export function isBoolean(value: unknown): value is boolean {
  return typeof value === 'boolean';
}

export function integerIn(min: number, max: number): Check<number> {
  return (value): value is number =>
    Number.isInteger(value) && min <= Number(value) && Number(value) <= max;
}

export function oneOf<const T extends readonly unknown[]>(...options: T): Check<T[number]> {
  return (value): value is T[number] => options.includes(value);
}

export function nullable<T>(check: Check<T>): Check<T | null> {
  return (value): value is T | null => value === null || check(value);
}

export function arrayOf<T>(check: Check<T>): Check<readonly T[]> {
  return (value): value is readonly T[] =>
    Array.isArray(value) && value.every((item) => check(item));
}

export function objectOf<S extends Shape>(shape: S): Check<ShapeOf<S>> {
  return (value): value is ShapeOf<S> =>
    typeof value === 'object' &&
    value !== null &&
    Object.entries(shape).every(([key, check]) => check(Reflect.get(value, key)));
}

export function anyOf<const C extends readonly Check<unknown>[]>(
  ...checks: C
): Check<Checked<C[number]>> {
  return (value): value is Checked<C[number]> => checks.some((check) => check(value));
}

export function parseJson(raw: string | null): unknown {
  if (raw === null) {
    return undefined;
  }
  try {
    const value: unknown = JSON.parse(raw);
    return value;
  } catch {
    return undefined;
  }
}
