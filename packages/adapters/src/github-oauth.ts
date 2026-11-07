import { ChefError } from '../../domain/src/errors.js';
import type { OAuthIdentity, OAuthProvider } from '../../domain/src/ports.js';

const SCOPES = 'read:user';
const DEFAULT_TIMEOUT_MS = 10_000;

export interface GitHubOAuthOptions {
  clientId: string;
  clientSecret: string;
  redirectUri: string;
  authorizeBase?: string;
  apiBase?: string;
  timeoutMs?: number;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/**
 * GitHub OAuth web flow. The access token never leaves this class: it is used
 * immediately to read the identity and then discarded, never logged or stored.
 */
export class GitHubOAuth implements OAuthProvider {
  readonly name = 'github';
  private readonly authorizeBase: string;
  private readonly apiBase: string;
  private readonly timeoutMs: number;

  constructor(private readonly options: GitHubOAuthOptions) {
    if (!options.clientId || !options.clientSecret || !options.redirectUri)
      throw new ChefError('INVALID_INPUT', 'GitHub OAuth is not configured');
    this.authorizeBase = options.authorizeBase ?? 'https://github.com';
    this.apiBase = options.apiBase ?? 'https://api.github.com';
    this.timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  }

  authorizeUrl(state: string): string {
    if (state === '')
      throw new ChefError('INVALID_INPUT', 'OAuth state is required');
    const url = new URL('/login/oauth/authorize', this.authorizeBase);
    url.searchParams.set('client_id', this.options.clientId);
    url.searchParams.set('redirect_uri', this.options.redirectUri);
    url.searchParams.set('scope', SCOPES);
    url.searchParams.set('state', state);
    url.searchParams.set('allow_signup', 'false');
    return url.toString();
  }

  async exchange(code: string): Promise<OAuthIdentity> {
    if (code.trim() === '')
      throw new ChefError('INVALID_INPUT', 'Authorization code is required');
    const accessToken = await this.requestAccessToken(code);
    const identity = await this.requestIdentity(accessToken);
    return identity;
  }

  private async requestAccessToken(code: string): Promise<string> {
    const url = new URL('/login/oauth/access_token', this.authorizeBase);
    let response: Response;
    try {
      response = await fetch(url, {
        method: 'POST',
        headers: {
          accept: 'application/json',
          'content-type': 'application/json',
        },
        body: JSON.stringify({
          client_id: this.options.clientId,
          client_secret: this.options.clientSecret,
          code,
          redirect_uri: this.options.redirectUri,
        }),
        signal: AbortSignal.timeout(this.timeoutMs),
      });
    } catch {
      throw new ChefError(
        'UPSTREAM_UNAVAILABLE',
        'GitHub token endpoint is unreachable',
      );
    }
    if (!response.ok)
      throw new ChefError(
        'UPSTREAM_ERROR',
        `GitHub token endpoint returned ${response.status}`,
      );
    const body: unknown = await response.json();
    if (
      !isRecord(body) ||
      typeof body.access_token !== 'string' ||
      body.access_token === ''
    )
      throw new ChefError(
        'UPSTREAM_ERROR',
        'GitHub did not return an access token',
      );
    return body.access_token;
  }

  private async requestIdentity(accessToken: string): Promise<OAuthIdentity> {
    const url = new URL('/user', this.apiBase);
    let response: Response;
    try {
      response = await fetch(url, {
        headers: {
          accept: 'application/vnd.github+json',
          authorization: `Bearer ${accessToken}`,
          'x-github-api-version': '2022-11-28',
        },
        signal: AbortSignal.timeout(this.timeoutMs),
      });
    } catch {
      throw new ChefError(
        'UPSTREAM_UNAVAILABLE',
        'GitHub user endpoint is unreachable',
      );
    }
    if (!response.ok)
      throw new ChefError(
        'UPSTREAM_ERROR',
        `GitHub user endpoint returned ${response.status}`,
      );
    const body: unknown = await response.json();
    if (
      !isRecord(body) ||
      typeof body.id !== 'number' ||
      typeof body.login !== 'string' ||
      body.login === ''
    )
      throw new ChefError('UPSTREAM_ERROR', 'GitHub returned an invalid user');
    return { providerUserId: String(body.id), login: body.login };
  }
}
