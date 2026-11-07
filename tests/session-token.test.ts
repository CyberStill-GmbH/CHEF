import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { SessionTokenService } from '../packages/adapters/src/session-token.js';
import { ChefError } from '../packages/domain/src/errors.js';

const SECRET = 's'.repeat(32);

function service(start = 1_000_000) {
  const state = { now: start };
  const sessions = new SessionTokenService({
    secret: SECRET,
    nowMs: () => state.now,
    ttlSeconds: 60,
    nonceTtlSeconds: 30,
  });
  return { sessions, state };
}

describe('session tokens', () => {
  it('issues a token that round-trips to the same user', () => {
    const { sessions } = service();
    assert.equal(sessions.verify(sessions.issue('github:42')), 'github:42');
  });

  it('rejects a modified payload', () => {
    const { sessions } = service();
    const token = sessions.issue('github:42');
    const parts = token.split('.');
    const forged = `${parts[0]}.${Buffer.from(
      JSON.stringify({ kind: 'session', sub: 'github:1', exp: 9_999_999_999 }),
    ).toString('base64url')}.${parts[2]}`;
    assert.throws(
      () => sessions.verify(forged),
      (error: unknown) =>
        error instanceof ChefError && error.code === 'UNAUTHORIZED',
    );
  });

  it('rejects a truncated signature', () => {
    const { sessions } = service();
    const token = sessions.issue('github:42');
    assert.throws(
      () => sessions.verify(`${token.slice(0, -2)}xx`),
      (error: unknown) =>
        error instanceof ChefError && error.code === 'UNAUTHORIZED',
    );
  });

  it('expires a session after its ttl', () => {
    const { sessions, state } = service();
    const token = sessions.issue('github:42');
    state.now += 61_000;
    assert.throws(
      () => sessions.verify(token),
      (error: unknown) =>
        error instanceof ChefError && error.code === 'UNAUTHORIZED',
    );
  });

  it('does not accept a nonce as a session', () => {
    const { sessions } = service();
    const nonce = sessions.issueNonce('oauth-state');
    assert.throws(
      () => sessions.verify(nonce),
      (error: unknown) =>
        error instanceof ChefError && error.code === 'UNAUTHORIZED',
    );
  });

  it('rejects a nonce replayed for another purpose', () => {
    const { sessions } = service();
    const nonce = sessions.issueNonce('oauth-state');
    assert.doesNotThrow(() => sessions.verifyNonce(nonce, 'oauth-state'));
    assert.throws(
      () => sessions.verifyNonce(nonce, 'other'),
      (error: unknown) =>
        error instanceof ChefError && error.code === 'UNAUTHORIZED',
    );
  });

  it('refuses a short secret', () => {
    assert.throws(
      () => new SessionTokenService({ secret: 'short', nowMs: () => 0 }),
      (error: unknown) =>
        error instanceof ChefError && error.code === 'INVALID_INPUT',
    );
  });
});
