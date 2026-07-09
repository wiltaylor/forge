import { describe, expect, it } from 'vitest';
import { buildEventsUrl, buildWsUrl } from '../src/index';

describe('buildEventsUrl', () => {
  it('builds a bare same-origin URL without token or topics', () => {
    expect(buildEventsUrl('', null)).toBe('/api/events');
  });

  it('carries the token as ?token= (EventSource cannot set headers)', () => {
    expect(buildEventsUrl('', 'jwt-abc')).toBe('/api/events?token=jwt-abc');
  });

  it('comma-joins topics per the contract filter', () => {
    expect(buildEventsUrl('http://localhost:8765', 'jwt', ['logs', 'metrics'])).toBe(
      'http://localhost:8765/api/events?token=jwt&topics=logs,metrics',
    );
  });

  it('includes topics without a token (auth-disabled mode)', () => {
    expect(buildEventsUrl('', null, ['logs'])).toBe('/api/events?topics=logs');
  });

  it('URL-encodes token and topic values', () => {
    expect(buildEventsUrl('', 'a+b', ['top/ic'])).toBe('/api/events?token=a%2Bb&topics=top%2Fic');
  });
});

describe('buildWsUrl', () => {
  it('maps http baseUrl to ws', () => {
    expect(buildWsUrl('http://localhost:8765', 'jwt')).toBe(
      'ws://localhost:8765/api/ws?token=jwt',
    );
  });

  it('maps https baseUrl to wss', () => {
    expect(buildWsUrl('https://forge.example.com', 'jwt')).toBe(
      'wss://forge.example.com/api/ws?token=jwt',
    );
  });

  it('omits the token query when no token is stored (auth-disabled mode)', () => {
    expect(buildWsUrl('http://localhost:8765', null)).toBe('ws://localhost:8765/api/ws');
  });

  it('URL-encodes the token', () => {
    expect(buildWsUrl('http://h', 'a b')).toBe('ws://h/api/ws?token=a%20b');
  });

  it('throws a clear error for same-origin use outside a browser', () => {
    expect(() => buildWsUrl('', 'jwt')).toThrow(/baseUrl is required/);
  });
});
