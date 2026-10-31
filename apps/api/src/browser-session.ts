import {
  createCipheriv,
  createDecipheriv,
  hkdfSync,
  randomBytes,
} from 'node:crypto';
import type { IncomingMessage, ServerResponse } from 'node:http';
import { ChefError } from '../../../packages/domain/src/errors.js';
import type { SessionService } from '../../../packages/domain/src/ports.js';

function cookie(request: IncomingMessage, name: string): string | undefined {
  const values = (request.headers.cookie ?? '')
    .split(';')
    .map((part) => part.trim())
    .filter((part) => part.startsWith(`${name}=`));
  if (values.length > 1)
    throw new ChefError('UNAUTHORIZED', 'Ambiguous browser session');
  return values[0]?.slice(name.length + 1);
}

/** Same-origin browser transport; bearer clients remain a separate supported path. */
export class BrowserSession {
  readonly origin: string;
  private readonly secure: boolean;
  private readonly sessionName: string;
  private readonly stateName: string;
  private readonly key: Buffer;

  constructor(
    origin: string,
    private readonly sessions: SessionService,
    secret: string,
  ) {
    const url = new URL(origin);
    if (
      url.origin !== origin ||
      url.username ||
      url.password ||
      !(
        url.protocol === 'https:' ||
        (url.protocol === 'http:' &&
          ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname))
      )
    )
      throw new Error(
        'CHEF_WEB_ORIGIN must be an HTTPS origin or HTTP loopback origin',
      );
    this.origin = origin;
    if (secret.length < 32)
      throw new Error('Browser cookie secret must have at least 32 characters');
    this.key = Buffer.from(
      hkdfSync('sha256', secret, '', 'chef-browser-cookie/v1', 32),
    );
    this.secure = url.protocol === 'https:';
    this.sessionName = this.secure ? '__Host-chef-session' : 'chef-session';
    this.stateName = this.secure ? '__Host-chef-oauth' : 'chef-oauth';
  }

  checkRequest(request: IncomingMessage): void {
    if (
      request.headers['x-chef-request'] !== '1' ||
      (request.method !== 'GET' && request.headers.origin !== this.origin)
    )
      throw new ChefError('FORBIDDEN', 'Browser request origin is not allowed');
  }

  token(request: IncomingMessage): string | undefined {
    const value = cookie(request, this.sessionName);
    if (value !== undefined) this.checkRequest(request);
    return value === undefined ? undefined : this.open(this.sessionName, value);
  }

  private seal(name: string, value: string): string {
    const iv = randomBytes(12);
    const cipher = createCipheriv('aes-256-gcm', this.key, iv);
    cipher.setAAD(Buffer.from(`${this.origin}|${name}|b1`));
    const encrypted = Buffer.concat([
      cipher.update(value, 'utf8'),
      cipher.final(),
    ]);
    return [
      'b1',
      iv.toString('base64url'),
      encrypted.toString('base64url'),
      cipher.getAuthTag().toString('base64url'),
    ].join('.');
  }

  private open(name: string, value: string): string {
    try {
      const [version, ivText, encryptedText, tagText, extra] = value.split('.');
      if (
        version !== 'b1' ||
        extra !== undefined ||
        value.length > 4096 ||
        !ivText ||
        !encryptedText ||
        !tagText ||
        ![ivText, encryptedText, tagText].every((part) =>
          /^[A-Za-z0-9_-]+$/.test(part),
        )
      )
        throw new Error('Invalid cookie');
      const iv = Buffer.from(ivText, 'base64url'),
        tag = Buffer.from(tagText, 'base64url');
      if (iv.length !== 12 || tag.length !== 16)
        throw new Error('Invalid cookie');
      const decipher = createDecipheriv('aes-256-gcm', this.key, iv);
      decipher.setAAD(Buffer.from(`${this.origin}|${name}|b1`));
      decipher.setAuthTag(tag);
      return Buffer.concat([
        decipher.update(Buffer.from(encryptedText, 'base64url')),
        decipher.final(),
      ]).toString('utf8');
    } catch {
      throw new ChefError('UNAUTHORIZED', 'Invalid browser session');
    }
  }

  private setCookie(
    response: ServerResponse,
    name: string,
    value: string,
    maxAge: number,
  ): void {
    const previous = response.getHeader('set-cookie');
    const headers = Array.isArray(previous)
      ? previous
      : previous === undefined
        ? []
        : [String(previous)];
    response.setHeader('set-cookie', [
      ...headers,
      `${name}=${value === '' ? '' : this.seal(name, value)}; Path=/; HttpOnly; SameSite=Lax; Max-Age=${maxAge}${this.secure ? '; Secure' : ''}`,
    ]);
  }

  start(request: IncomingMessage, response: ServerResponse): string {
    this.checkRequest(request);
    const nonce = randomBytes(24).toString('hex');
    const state = this.sessions.issueNonce(`oauth-browser:${nonce}`);
    this.setCookie(response, this.stateName, `${nonce}.${state}`, 600);
    return state;
  }

  verifyState(request: IncomingMessage, state: string): void {
    const value = this.open(
      this.stateName,
      cookie(request, this.stateName) ?? '',
    );
    const separator = value.indexOf('.');
    const nonce = value.slice(0, separator);
    if (!/^[0-9a-f]{48}$/.test(nonce) || value.slice(separator + 1) !== state)
      throw new ChefError('UNAUTHORIZED', 'Invalid browser OAuth state');
    this.sessions.verifyNonce(state, `oauth-browser:${nonce}`);
  }

  finish(response: ServerResponse, userId: string): void {
    this.setCookie(response, this.stateName, '', 0);
    this.setCookie(
      response,
      this.sessionName,
      this.sessions.issue(userId),
      8 * 60 * 60,
    );
    this.redirect(response);
  }

  failure(response: ServerResponse): void {
    this.setCookie(response, this.stateName, '', 0);
    this.redirect(response, true);
  }

  private redirect(response: ServerResponse, failed = false): void {
    response.writeHead(303, {
      location: `${this.origin}/${failed ? '?auth=failed' : ''}`,
      'cache-control': 'no-store',
      'referrer-policy': 'no-referrer',
      'content-security-policy': "default-src 'none'; frame-ancestors 'none'",
    });
    response.end();
  }

  logout(request: IncomingMessage, response: ServerResponse): void {
    this.checkRequest(request);
    this.setCookie(response, this.sessionName, '', 0);
    this.setCookie(response, this.stateName, '', 0);
  }
}
