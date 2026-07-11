import { describe, it, expect } from 'vitest';
import { EMOJI, resolveEmoji, searchEmoji } from '../src/emoji';

describe('emoji table', () => {
  it('contains the staples', () => {
    for (const code of ['rocket', 'warning', '+1', 'fire', 'white_check_mark', 'sparkles'])
      expect(EMOJI[code], code).toBeTruthy();
  });
});

describe('resolveEmoji', () => {
  it('replaces known shortcodes and keeps unknown ones literal', () => {
    expect(resolveEmoji('go :rocket: now')).toBe('go \u{1F680} now');
    expect(resolveEmoji('a :not_a_real_code: b')).toBe('a :not_a_real_code: b');
    expect(resolveEmoji('10:30 meeting :+1:')).toBe('10:30 meeting \u{1F44D}');
    expect(resolveEmoji('no emoji here')).toBe('no emoji here');
  });
  it('consumer extras win over builtins', () => {
    expect(resolveEmoji(':rocket:', { rocket: 'X' })).toBe('X');
    expect(resolveEmoji(':custom_thing:', { custom_thing: 'Y' })).toBe('Y');
  });
});

describe('searchEmoji', () => {
  it('prefix-matches with a limit', () => {
    const hits = searchEmoji('roc');
    expect(hits.some((h) => h.code === 'rocket')).toBe(true);
    expect(searchEmoji('s', undefined, 3)).toHaveLength(3);
    expect(searchEmoji('zzzznothing')).toHaveLength(0);
  });
  it('includes extras first', () => {
    const hits = searchEmoji('roc', { rock_on: 'Z' });
    expect(hits[0]!.code).toBe('rock_on');
  });
});
