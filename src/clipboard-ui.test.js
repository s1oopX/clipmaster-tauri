import { describe, expect, it } from 'vitest';
import { isWebUrl } from './lib/clipboard-ui.js';

describe('clipboard URL detection', () => {
  it('accepts normal web links and rejects unsafe local or ambiguous links', () => {
    expect(isWebUrl(' https://example.com/docs?q=1#read ')).toBe(true);
    expect(isWebUrl('http://docs.example.com')).toBe(true);

    for (const value of [
      '',
      'example.com',
      'https://example',
      'https://localhost',
      'https://127.0.0.1',
      'https://10.0.0.2',
      'https://192.168.1.2',
      'https://172.16.0.1',
      'https://user:pass@example.com',
      'https://example.com\\@evil.test/',
      'https://example.com with words',
      'javascript:alert(1)',
      'file:///C:/temp/a.txt',
    ]) {
      expect(isWebUrl(value), value).toBe(false);
    }
  });
});
