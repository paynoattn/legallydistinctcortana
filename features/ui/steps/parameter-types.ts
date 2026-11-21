import { defineParameterType } from '@cucumber/cucumber';

export interface Place {
  readonly number: number;
  readonly count: number;
}

export interface Timing {
  readonly startsInMs: number;
  readonly speechMs: number;
}

defineParameterType({
  name: 'place',
  regexp: /(\d{1,9}) of (\d{1,9})/,
  transformer: (number: string, count: string): Place => ({
    number: Number(number),
    count: Number(count),
  }),
});

defineParameterType({
  name: 'timing',
  regexp: /(\d{1,9}) ms with (\d{1,9}) ms/,
  transformer: (startsInMs: string, speechMs: string): Timing => ({
    startsInMs: Number(startsInMs),
    speechMs: Number(speechMs),
  }),
});

defineParameterType({
  name: 'pair',
  regexp: /(\d{1,9}) and (\d{1,9})/,
  transformer: (first: string, second: string): readonly number[] => [
    Number(first),
    Number(second),
  ],
});
