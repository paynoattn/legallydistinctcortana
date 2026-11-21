export interface Sentence {
  readonly text: string;
  readonly startsParagraph: boolean;
}

export type Selection =
  | { readonly kind: 'waiting' }
  | { readonly kind: 'ready'; readonly sentences: readonly Sentence[] }
  | { readonly kind: 'failed'; readonly message: string };

export type ReadingStatus =
  | { readonly kind: 'reading' }
  | { readonly kind: 'finished' }
  | { readonly kind: 'stopped' }
  | { readonly kind: 'failed'; readonly message: string };

export interface SentenceReport {
  readonly index: number;
  readonly startsInMs: number;
  readonly speechMs: number;
}

export const WAITING: Selection = { kind: 'waiting' };

export interface NumberedSentence {
  readonly index: number;
  readonly text: string;
}

export function paragraphsOf(
  sentences: readonly Sentence[],
): readonly (readonly NumberedSentence[])[] {
  const paragraphs: NumberedSentence[][] = [];
  sentences.forEach((sentence, index) => {
    const numbered = { index, text: sentence.text };
    const last = paragraphs.at(-1);
    if (sentence.startsParagraph || last === undefined) {
      paragraphs.push([numbered]);
    } else {
      last.push(numbered);
    }
  });
  return paragraphs;
}

export function wordsOf(text: string): readonly string[] {
  return text.split(/\s/).filter((word) => word !== '');
}
