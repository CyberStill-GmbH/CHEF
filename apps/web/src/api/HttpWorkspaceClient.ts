import { decodeSnapshot } from '../contracts/decodeSnapshot';
import type { ProjectSummary, WorkspaceClient } from './WorkspaceClient';

export class ApiError extends Error {
  constructor(readonly status: number) {
    super(
      status === 401
        ? 'La sesión ha caducado. Inicia sesión de nuevo.'
        : status === 403
          ? 'Tu cuenta no tiene permiso para esta operación.'
          : status === 404
            ? 'El recurso no está disponible para tu cuenta.'
            : status === 413
              ? 'El archivo supera el límite de la importación.'
              : status === 422
                ? 'La importación contiene servicios fuera del scope autorizado.'
                : status === 503
                  ? 'El servicio no está disponible. Inténtalo de nuevo.'
                  : 'No se pudo completar la operación. Inténtalo de nuevo.',
    );
  }
}

export function operationError(error: unknown): string {
  return error instanceof ApiError
    ? error.message
    : 'No se pudo conectar con la API o validar su respuesta.';
}

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new ApiError(502);
  return value as Record<string, unknown>;
}

function projectSummary(value: unknown): ProjectSummary {
  const envelope = record(value),
    project = record(envelope['project']);
  if (
    typeof project['id'] !== 'string' ||
    !/^project:[0-9a-f]{64}$/.test(project['id']) ||
    typeof project['name'] !== 'string' ||
    typeof project['ownerId'] !== 'string' ||
    typeof project['createdAt'] !== 'string' ||
    !Number.isFinite(Date.parse(project['createdAt'])) ||
    !['owner', 'member'].includes(String(envelope['role']))
  )
    throw new ApiError(502);
  return {
    project: {
      id: project['id'],
      name: project['name'],
      ownerId: project['ownerId'],
      createdAt: project['createdAt'],
    },
    role: envelope['role'] as 'owner' | 'member',
  };
}

/** Same-origin /api proxy only. The browser handles the HttpOnly session cookie. */
export class HttpWorkspaceClient implements WorkspaceClient {
  constructor(
    private readonly fetcher: typeof fetch = (input, init) =>
      globalThis.fetch(input, init),
  ) {}

  private request(path: string, options: RequestInit = {}) {
    return this.fetcher(`/api${path}`, {
      ...options,
      credentials: 'same-origin',
      cache: 'no-store',
      redirect: 'error',
      headers: { 'x-chef-request': '1', ...options.headers },
    });
  }

  async session(signal?: AbortSignal) {
    const response = await this.request('/auth/session', {
      signal: signal ?? null,
    });
    if (response.status === 401) return null;
    if (!response.ok) throw new ApiError(response.status);
    const result = record(await response.json());
    if (typeof result['userId'] !== 'string' || result['userId'] === '')
      throw new ApiError(502);
    return { userId: result['userId'] };
  }

  async startSignIn() {
    const response = await this.request('/auth/github/browser/start', {
      method: 'POST',
    });
    if (!response.ok) throw new ApiError(response.status);
    const result = record(await response.json());
    if (typeof result['authorizeUrl'] !== 'string') throw new ApiError(502);
    const url = new URL(result['authorizeUrl']);
    if (
      url.origin !== 'https://github.com' ||
      url.pathname !== '/login/oauth/authorize' ||
      url.username ||
      url.password
    )
      throw new ApiError(502);
    return url.href;
  }

  async logout() {
    const response = await this.request('/auth/logout', { method: 'POST' });
    if (!response.ok) throw new ApiError(response.status);
  }

  async listProjects(signal?: AbortSignal) {
    const response = await this.request('/projects', {
      signal: signal ?? null,
    });
    if (!response.ok) throw new ApiError(response.status);
    const result = record(await response.json());
    if (!Array.isArray(result['projects'])) throw new ApiError(502);
    return result['projects'].map(projectSummary);
  }

  async createProject(name: string, signal?: AbortSignal) {
    const response = await this.request('/projects', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ name }),
      signal: signal ?? null,
    });
    if (!response.ok) throw new ApiError(response.status);
    return projectSummary(await response.json());
  }

  async loadProjectSnapshot(projectId: string, signal?: AbortSignal) {
    const response = await this.request(
      `/projects/${encodeURIComponent(projectId)}/snapshot`,
      { signal: signal ?? null },
    );
    if (response.status === 404) return null;
    if (!response.ok) throw new ApiError(response.status);
    return decodeSnapshot(await response.json());
  }

  async importXml(projectId: string, file: File, signal?: AbortSignal) {
    const response = await this.request(
      `/projects/${encodeURIComponent(projectId)}/imports`,
      {
        method: 'POST',
        headers: { 'content-type': 'application/xml' },
        body: file,
        signal: signal ?? null,
      },
    );
    if (!response.ok) throw new ApiError(response.status);
    const result = record(await response.json());
    if (
      typeof result['runId'] !== 'string' ||
      !/^run:[0-9a-f]{64}$/.test(result['runId']) ||
      ['observations', 'assets', 'relationships'].some(
        (key) => !Number.isSafeInteger(result[key]) || Number(result[key]) < 0,
      )
    )
      throw new ApiError(502);
  }
}
