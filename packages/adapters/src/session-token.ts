import { createHmac, timingSafeEqual } from 'node:crypto';

import { ChefError } from '../../domain/src/errors.js';
import type { SessionService } from '../../domain/src/ports.js';

const VERSION = 'v1';
const MIN_SECRET = 32;
const DEFAULT_TTL_SECONDS = 8 * 60 * 60;
const DEFAULT_NONCE_TTL_SECONDS = 10 * 60;

interface Claims {
  kind: 'session' | 'nonce';
  sub: string;
  exp: number;
}

export interface SessionTokenOptions {
  secret: string;
  nowMs: () => number;
  ttlSeconds?: number;
  nonceTtlSeconds?: number;
}

function isClaims(value: unknown): value is Claims {
  if (typeof value !== 'object' || value === null) return false;
  const record = value as Record<string, unknown>;
  return (
    (record.kind === 'session' || record.kind === 'nonce') &&
    typeof record.sub === 'string' &&
    record.sub !== '' &&
    typeof record.exp === 'number' &&
    Number.isFinite(record.exp)
  );
}

/**
 * HMAC-SHA256 stateless tokens. Signed, not encrypted: a token carries only a
 * user id and an expiry, never evidence, scope or client data.
 */
export class SessionTokenService implements SessionService {
  private readonly secret: string;
  private readonly ttlSeconds: number;
  private readonly nonceTtlSeconds: number;

  constructor(private readonly options: SessionTokenOptions) {
    if (options.secret.length < MIN_SECRET)
      throw new ChefError(
        'INVALID_INPUT',
        'Session secret must be at least 32 characters',
      );
    this.secret = options.secret;
    this.ttlSeconds = options.ttlSeconds ?? DEFAULT_TTL_SECONDS;
    this.nonceTtlSeconds = options.nonceTtlSeconds ?? DEFAULT_NONCE_TTL_SECONDS;
  }

  private sign(body: string): Buffer {
    return createHmac('sha256', this.secret).update(body).digest();
  }

  private mint(claims: Claims): string {
    const body = `${VERSION}.${Buffer.from(JSON.stringify(claims), 'utf8').toString('base64url')}`;
    return `${body}.${this.sign(body).toString('base64url')}`;
  }

  private open(token: string, kind: Claims['kind']): Claims {
    const parts = token.split('.');
    const payload = parts[1];
    const signature = parts[2];
    if (parts.length !== 3 || parts[0] !== VERSION || !payload || !signature)
      throw new ChefError('UNAUTHORIZED', 'Invalid credentials');
    const body = `${VERSION}.${payload}`;
    const expected = this.sign(body);
    const actual = Buffer.from(signature, 'base64url');
    if (actual.length !== expected.length || !timingSafeEqual(actual, expected))
      throw new ChefError('UNAUTHORIZED', 'Invalid credentials');
    let decoded: unknown;
    try {
      decoded = JSON.parse(Buffer.from(payload, 'base64url').toString('utf8'));
    } catch {
      throw new ChefError('UNAUTHORIZED', 'Invalid credentials');
    }
    if (!isClaims(decoded) || decoded.kind !== kind)
      throw new ChefError('UNAUTHORIZED', 'Invalid credentials');
    if (decoded.exp <= Math.floor(this.options.nowMs() / 1000))
      throw new ChefError('UNAUTHORIZED', 'Credentials expired');
    return decoded;
  }

  issue(userId: string): string {
    if (userId.trim() === '')
      throw new ChefError('UNAUTHORIZED', 'An authenticated user is required');
    return this.mint({
      kind: 'session',
      sub: userId,
      exp: Math.floor(this.options.nowMs() / 1000) + this.ttlSeconds,
    });
  }

  verify(token: string): string {
    return this.open(token, 'session').sub;
  }

  issueNonce(purpose: string): string {
    return this.mint({
      kind: 'nonce',
      sub: purpose,
      exp: Math.floor(this.options.nowMs() / 1000) + this.nonceTtlSeconds,
    });
  }

  verifyNonce(nonce: string, purpose: string): void {
    const claims = this.open(nonce, 'nonce');
    if (claims.sub !== purpose)
      throw new ChefError('UNAUTHORIZED', 'Invalid credentials');
  }
}
