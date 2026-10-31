import { describe, expect, it, vi } from 'vitest';
import fixture from '../../../../docs/contracts/examples/snapshot.json';
import { ApiError, HttpWorkspaceClient } from '../api/HttpWorkspaceClient';

const project = {
  project: {
    id: `project:${'a'.repeat(64)}`,
    name: 'Proyecto',
    ownerId: 'github:1',
    createdAt: '2026-10-30T00:00:00Z',
  },
  role: 'owner',
};
function response(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), { status });
}

describe('HttpWorkspaceClient', () => {
  it('keeps the browser fetch receiver when using the default transport', async () => {
    const previous = globalThis.fetch;
    globalThis.fetch = function (this: typeof globalThis) {
      expect(this).toBe(globalThis);
      return Promise.resolve(response({ userId: 'github:1' }));
    };
    try {
      expect(await new HttpWorkspaceClient().session()).toEqual({
        userId: 'github:1',
      });
    } finally {
      globalThis.fetch = previous;
    }
  });
  it('uses same-origin cookies and no bearer token or browser storage', async () => {
    const fetcher = vi
      .fn<typeof fetch>()
      .mockResolvedValue(response({ userId: 'github:1' }));
    const client = new HttpWorkspaceClient(fetcher);
    expect(await client.session()).toEqual({ userId: 'github:1' });
    expect(fetcher).toHaveBeenCalledWith(
      '/api/auth/session',
      expect.objectContaining({
        credentials: 'same-origin',
        cache: 'no-store',
        headers: { 'x-chef-request': '1' },
      }),
    );
    expect(localStorage.length).toBe(0);
    expect(sessionStorage.length).toBe(0);
  });

  it('handles expired sessions and redacts server errors', async () => {
    const fetcher = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(response({}, 401))
      .mockResolvedValueOnce(response({ message: 'private stack trace' }, 503));
    const client = new HttpWorkspaceClient(fetcher);
    expect(await client.session()).toBeNull();
    await expect(client.listProjects()).rejects.toThrow(
      'El servicio no está disponible.',
    );
  });

  it('validates projects and snapshots before presenting them', async () => {
    const fetcher = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(response({ projects: [project] }))
      .mockResolvedValueOnce(response(fixture))
      .mockResolvedValueOnce(
        response({ projects: [{ project: { id: 'invalid' } }] }),
      );
    const client = new HttpWorkspaceClient(fetcher);
    expect(await client.listProjects()).toEqual([project]);
    expect(await client.loadProjectSnapshot(project.project.id)).toEqual(
      fixture,
    );
    await expect(client.listProjects()).rejects.toBeInstanceOf(ApiError);
  });

  it('treats missing snapshots as unavailable and rejects hostile OAuth redirects', async () => {
    const fetcher = vi
      .fn<typeof fetch>()
      .mockResolvedValueOnce(response({}, 404))
      .mockResolvedValueOnce(
        response({ authorizeUrl: 'https://foreign.invalid/steal' }),
      );
    const client = new HttpWorkspaceClient(fetcher);
    expect(await client.loadProjectSnapshot(project.project.id)).toBeNull();
    await expect(client.startSignIn()).rejects.toBeInstanceOf(ApiError);
  });

  it('sends raw XML, encodes project IDs and propagates cancellation', async () => {
    const fetcher = vi.fn<typeof fetch>().mockResolvedValue(
      response({
        runId: `run:${'b'.repeat(64)}`,
        observations: 3,
        assets: 4,
        relationships: 2,
      }),
    );
    const client = new HttpWorkspaceClient(fetcher);
    const file = new File(['<nmaprun/>'], 'scan.xml', {
      type: 'application/xml',
    });
    const controller = new AbortController();
    await client.importXml(project.project.id, file, controller.signal);
    expect(fetcher).toHaveBeenCalledWith(
      `/api/projects/${encodeURIComponent(project.project.id)}/imports`,
      expect.objectContaining({
        body: file,
        method: 'POST',
        signal: controller.signal,
        headers: { 'x-chef-request': '1', 'content-type': 'application/xml' },
      }),
    );
  });
});
