import {
  createServer,
  type IncomingMessage,
  type ServerResponse,
} from 'node:http';

import type { CreateProject } from '../../../packages/application/src/create-project.js';
import type { ImportIntoProject } from '../../../packages/application/src/import-into-project.js';
import type { ListProjects } from '../../../packages/application/src/list-projects.js';
import type { ReadProjectSnapshot } from '../../../packages/application/src/read-project-snapshot.js';
import { ChefError } from '../../../packages/domain/src/errors.js';
import type { ScopePolicy } from '../../../packages/domain/src/model.js';
import type {
  OAuthProvider,
  SessionService,
} from '../../../packages/domain/src/ports.js';
import { BrowserSession } from './browser-session.js';

const DEFAULT_MAX_BODY_BYTES = 5 * 1024 * 1024;
const OAUTH_STATE_PURPOSE = 'oauth-state';
const BEARER = 'Bearer ';
const XML_TYPE = /^application\/(xml|octet-stream)$/;

export interface ApiDependencies {
  createProject: CreateProject;
  listProjects: ListProjects;
  importIntoProject: ImportIntoProject;
  readSnapshot: ReadProjectSnapshot;
  sessions: SessionService;
  oauth: OAuthProvider;
  scopePolicy: ScopePolicy;
  allowedOAuthUserIds?: ReadonlySet<string>;
  webOrigin?: string;
  browserCookieSecret?: string;
  maxBodyBytes?: number;
}

const SECURITY_HEADERS: Record<string, string> = {
  'cache-control': 'no-store',
  'x-content-type-options': 'nosniff',
  'content-security-policy': "default-src 'none'",
  'referrer-policy': 'no-referrer',
};

const STATUS: Record<string, number> = {
  INVALID_INPUT: 400,
  UNAUTHORIZED: 401,
  FORBIDDEN: 403,
  NOT_FOUND: 404,
  CONFLICT: 409,
  LIMIT_EXCEEDED: 413,
  OUT_OF_SCOPE: 422,
  CANCELLED: 499,
  UPSTREAM_ERROR: 502,
  UPSTREAM_UNAVAILABLE: 503,
};

function sendJson(
  response: ServerResponse,
  status: number,
  body: unknown,
): void {
  const payload = JSON.stringify(body);
  response.writeHead(status, {
    ...SECURITY_HEADERS,
    'content-type': 'application/json; charset=utf-8',
    'content-length': Buffer.byteLength(payload),
  });
  response.end(payload);
}

function fail(response: ServerResponse, error: unknown): void {
  if (response.destroyed || response.writableEnded) return;
  if (error instanceof ChefError) {
    sendJson(response, STATUS[error.code] ?? 500, {
      error: error.code,
      message: error.message,
    });
    return;
  }
  // Never echo an unexpected stack, path or query fragment to the client.
  sendJson(response, 500, {
    error: 'INTERNAL_ERROR',
    message: 'Unexpected server error',
  });
}

async function readBody(
  request: IncomingMessage,
  limit: number,
): Promise<Buffer> {
  const chunks: Buffer[] = [];
  let total = 0;
  for await (const chunk of request) {
    const buffer = Buffer.isBuffer(chunk)
      ? chunk
      : Buffer.from(chunk as string);
    total += buffer.length;
    if (total > limit)
      throw new ChefError('LIMIT_EXCEEDED', 'Request body exceeds limit');
    chunks.push(buffer);
  }
  return Buffer.concat(chunks);
}

async function readJson(
  request: IncomingMessage,
  limit: number,
): Promise<Record<string, unknown>> {
  const raw = await readBody(request, limit);
  if (raw.length === 0) return {};
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw.toString('utf8'));
  } catch {
    throw new ChefError('INVALID_INPUT', 'Request body is not valid JSON');
  }
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed))
    throw new ChefError('INVALID_INPUT', 'Request body must be a JSON object');
  return parsed as Record<string, unknown>;
}

function authenticate(
  request: IncomingMessage,
  dependencies: ApiDependencies,
  browser?: BrowserSession,
): string {
  const header = request.headers.authorization;
  if (
    header !== undefined &&
    (typeof header !== 'string' || !header.startsWith(BEARER))
  )
    throw new ChefError('UNAUTHORIZED', 'Missing bearer token');
  const token =
    typeof header === 'string'
      ? header.slice(BEARER.length).trim()
      : browser?.token(request);
  if (!token) throw new ChefError('UNAUTHORIZED', 'Missing session');
  const userId = dependencies.sessions.verify(token);
  if (dependencies.allowedOAuthUserIds) {
    const prefix = `${dependencies.oauth.name}:`;
    if (
      !userId.startsWith(prefix) ||
      !dependencies.allowedOAuthUserIds.has(userId.slice(prefix.length))
    )
      throw new ChefError(
        'FORBIDDEN',
        'Account is not enabled for this private instance',
      );
  }
  return userId;
}

function queryParam(url: URL, name: string): string {
  const value = url.searchParams.get(name);
  if (value === null || value === '')
    throw new ChefError('INVALID_INPUT', `Missing ${name} parameter`);
  return value;
}

function contentTypeOf(request: IncomingMessage): string {
  const header = request.headers['content-type'];
  return typeof header === 'string' ? header : '';
}

export function createChefApi(dependencies: ApiDependencies) {
  const limit = dependencies.maxBodyBytes ?? DEFAULT_MAX_BODY_BYTES;
  const browser =
    dependencies.webOrigin === undefined
      ? undefined
      : new BrowserSession(
          dependencies.webOrigin,
          dependencies.sessions,
          dependencies.browserCookieSecret ?? '',
        );
  const exchange = async (code: string) => {
    const identity = await dependencies.oauth.exchange(code);
    if (
      dependencies.allowedOAuthUserIds &&
      !dependencies.allowedOAuthUserIds.has(identity.providerUserId)
    )
      throw new ChefError(
        'FORBIDDEN',
        'GitHub account is not enabled for this private instance',
      );
    return {
      ...identity,
      userId: `${dependencies.oauth.name}:${identity.providerUserId}`,
    };
  };

  return createServer((request, response) => {
    void (async () => {
      try {
        const method = request.method ?? 'GET';
        const url = new URL(request.url ?? '/', 'http://localhost');
        const path = url.pathname;

        if (method === 'GET' && path === '/health') {
          sendJson(response, 200, { status: 'ok' });
          return;
        }

        if (method === 'GET' && path === '/auth/session') {
          sendJson(response, 200, {
            userId: authenticate(request, dependencies, browser),
          });
          return;
        }

        if (method === 'POST' && path === '/auth/logout') {
          if (!browser)
            throw new ChefError(
              'UPSTREAM_UNAVAILABLE',
              'Browser sign-in is not configured',
            );
          browser.logout(request, response);
          sendJson(response, 200, { status: 'signed-out' });
          return;
        }

        if (method === 'POST' && path === '/auth/github/browser/start') {
          if (!browser)
            throw new ChefError(
              'UPSTREAM_UNAVAILABLE',
              'Browser sign-in is not configured',
            );
          const state = browser.start(request, response);
          sendJson(response, 200, {
            authorizeUrl: dependencies.oauth.authorizeUrl(state),
          });
          return;
        }

        if (method === 'POST' && path === '/auth/github/start') {
          const state = dependencies.sessions.issueNonce(OAUTH_STATE_PURPOSE);
          sendJson(response, 200, {
            provider: dependencies.oauth.name,
            state,
            authorizeUrl: dependencies.oauth.authorizeUrl(state),
          });
          return;
        }

        if (method === 'GET' && path === '/auth/github/callback') {
          const state = queryParam(url, 'state');
          // Compatibility mode is selected only after verifying the signed CLI purpose.
          try {
            dependencies.sessions.verifyNonce(state, OAUTH_STATE_PURPOSE);
          } catch (failure) {
            if (!browser) throw failure;
            try {
              browser.verifyState(request, state);
              const identity = await exchange(queryParam(url, 'code'));
              browser.finish(response, identity.userId);
            } catch {
              browser.failure(response);
            }
            return;
          }
          const code = queryParam(url, 'code');
          const identity = await exchange(code);
          const userId = identity.userId;
          sendJson(response, 200, {
            userId,
            login: identity.login,
            token: dependencies.sessions.issue(userId),
          });
          return;
        }

        if (method === 'GET' && path === '/projects') {
          const userId = authenticate(request, dependencies, browser);
          sendJson(response, 200, {
            projects: await dependencies.listProjects.execute(userId),
          });
          return;
        }

        if (method === 'POST' && path === '/projects') {
          const userId = authenticate(request, dependencies, browser);
          const body = await readJson(request, limit);
          const name = body.name;
          if (typeof name !== 'string')
            throw new ChefError('INVALID_INPUT', 'Project name is required');
          const project = await dependencies.createProject.execute({
            name,
            ownerId: userId,
          });
          sendJson(response, 201, { project, role: 'owner' });
          return;
        }

        const importMatch = /^\/projects\/([^/]+)\/imports$/.exec(path);
        if (method === 'POST' && importMatch) {
          const userId = authenticate(request, dependencies, browser);
          const projectId = decodeURIComponent(importMatch[1] ?? '');
          if (!XML_TYPE.test(contentTypeOf(request)))
            throw new ChefError('INVALID_INPUT', 'Import requires an XML body');
          const bytes = await readBody(request, limit);
          const cancellation = new AbortController();
          response.once('close', () => {
            if (!response.writableEnded) cancellation.abort();
          });
          const snapshot = await dependencies.importIntoProject.execute({
            projectId,
            userId,
            bytes: new Uint8Array(bytes),
            policy: dependencies.scopePolicy,
            signal: cancellation.signal,
          });
          sendJson(response, 201, {
            runId: snapshot.run.id,
            observations: snapshot.observations.length,
            assets: snapshot.assets.length,
            relationships: snapshot.relationships.length,
          });
          return;
        }

        const snapshotMatch = /^\/projects\/([^/]+)\/snapshot$/.exec(path);
        if (method === 'GET' && snapshotMatch) {
          const userId = authenticate(request, dependencies, browser);
          const projectId = decodeURIComponent(snapshotMatch[1] ?? '');
          const snapshot = await dependencies.readSnapshot.execute(
            projectId,
            userId,
          );
          sendJson(response, 200, snapshot);
          return;
        }

        sendJson(response, 404, {
          error: 'NOT_FOUND',
          message: 'Unknown endpoint',
        });
      } catch (error) {
        fail(response, error);
      }
    })();
  });
}
