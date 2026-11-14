import { describe, expect, it } from 'vitest';
import { isMoodId, MOODS, moodForKey, moodLabel } from './mood.ts';

describe('moods', () => {
  it('offers nine moods with distinct ids, labels and hints', () => {
    expect(MOODS).toHaveLength(9);
    expect(new Set(MOODS.map((mood) => mood.id)).size).toBe(9);
    expect(new Set(MOODS.map((mood) => mood.label)).size).toBe(9);
    expect(MOODS.every((mood) => mood.hint !== '')).toBe(true);
    expect(MOODS[0]?.id).toBe('neutral');
  });

  it('words every mood the way the menu shows it', () => {
    expect(MOODS.map((mood) => [mood.id, mood.label, mood.hint])).toEqual([
      ['neutral', 'Neutral', 'Plain and even'],
      ['calm', 'Calm', 'Slower and softer'],
      ['cheerful', 'Cheerful', 'Bright and a little quicker'],
      ['alert', 'Alert', 'Crisp, to get attention'],
      ['urgent', 'Urgent', 'Fast and loud'],
      ['angry', 'Angry', 'Clipped and forceful'],
      ['mad', 'Mad', 'Angry, turned all the way up'],
      ['whisper', 'Whisper', 'Quiet and slow'],
      ['deadpan', 'Deadpan', 'Flat and unimpressed'],
    ]);
  });

  it.each(MOODS.map((mood) => mood.id))('recognises %s as a mood id', (id) => {
    expect(isMoodId(id)).toBe(true);
  });

  it.each(['', 'Calm', 'sad', 'toString'])('rejects %j as a mood id', (value) => {
    expect(isMoodId(value)).toBe(false);
  });

  it('labels a mood', () => {
    expect(moodLabel('whisper')).toBe('Whisper');
    expect(moodLabel('neutral')).toBe('Neutral');
  });

  it.each([
    ['1', 'neutral'],
    ['5', 'urgent'],
    ['9', 'deadpan'],
  ])('maps key %s to %s', (key, id) => {
    expect(moodForKey(key)?.id).toBe(id);
  });

  it.each(['0', '10', 'a', '', '11', '1a', 'a1', ' 1', '١'])('maps key %j to nothing', (key) => {
    expect(moodForKey(key)).toBeUndefined();
  });
});
