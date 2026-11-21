const DIGIT_WEIGHT = 1.4;
const PAUSE_WEIGHT = 1.3;
const WORD_WEIGHT = 0.25;
const PAUSE_AFTER = /[,;:—–]["'”’)]*$/;
const VOWEL_GROUPS = /[aeiouy]+/g;
const SILENT_E = /[^aeiouy]e$/;
const SOUNDED_LE = /[^aeiouy]le$/;

function silentE(letters: string): number {
  return SILENT_E.test(letters) && !SOUNDED_LE.test(letters) ? 1 : 0;
}

function spelledSyllables(letters: string): number {
  if (letters === '') {
    return 0;
  }
  const groups = letters.match(VOWEL_GROUPS)?.length ?? 0;
  return Math.max(1, groups - silentE(letters));
}

function syllables(word: string): number {
  const letters = word.toLowerCase().replace(/[^a-z0-9]/g, '');
  return /^\d+$/.test(letters) ? letters.length * DIGIT_WEIGHT : spelledSyllables(letters);
}

export function wordWeight(word: string): number {
  return syllables(word) + WORD_WEIGHT + (PAUSE_AFTER.test(word) ? PAUSE_WEIGHT : 0);
}

export function wordStarts(words: readonly string[], speechMs: number): readonly number[] {
  const weights = words.map(wordWeight);
  const total = weights.reduce((sum, weight) => sum + weight, 0);
  const perUnit = speechMs / total;
  let elapsed = 0;
  return weights.map((weight) => {
    const start = elapsed;
    elapsed += weight * perUnit;
    return start;
  });
}

export function wordAt(starts: readonly number[], elapsedMs: number): number | null {
  if (starts.length === 0 || elapsedMs < 0) {
    return null;
  }
  const next = starts.findIndex((start) => start > elapsedMs);
  return next === -1 ? starts.length - 1 : Math.max(0, next - 1);
}
