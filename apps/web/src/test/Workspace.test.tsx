import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import fixture from '../../../../docs/contracts/examples/snapshot.json';
import { ApiError } from '../api/HttpWorkspaceClient';
import type { ProjectSummary, WorkspaceClient } from '../api/WorkspaceClient';
import { decodeSnapshot } from '../contracts/decodeSnapshot';
import { Workspace } from '../Workspace';

const a: ProjectSummary = {
  project: {
    id: `project:${'a'.repeat(64)}`,
    name: 'Proyecto Alfa',
    ownerId: 'github:1',
    createdAt: '2026-10-30T00:00:00Z',
  },
  role: 'owner',
};
const b: ProjectSummary = {
  ...a,
  project: {
    ...a.project,
    id: `project:${'b'.repeat(64)}`,
    name: 'Proyecto Beta',
  },
};
const snapshot = decodeSnapshot(structuredClone(fixture));
function client(overrides: Partial<WorkspaceClient> = {}): WorkspaceClient {
  return {
    session: async () => ({ userId: 'github:1' }),
    startSignIn: async () =>
      'https://github.com/login/oauth/authorize?state=fixture',
    logout: async () => {},
    listProjects: async () => [a],
    createProject: async () => a,
    loadProjectSnapshot: async () => snapshot,
    importXml: async () => {},
    ...overrides,
  };
}

describe('Workspace', () => {
  it('starts signed out without showing synthetic results', async () => {
    const user = userEvent.setup(),
      navigate = vi.fn();
    render(
      <Workspace
        client={client({ session: async () => null })}
        onDemo={() => {}}
        navigate={navigate}
      />,
    );
    await user.click(
      await screen.findByRole('button', { name: 'Iniciar sesión con GitHub' }),
    );
    expect(navigate).toHaveBeenCalledWith(
      'https://github.com/login/oauth/authorize?state=fixture',
    );
    expect(
      screen.queryByRole('heading', { name: 'Resumen del snapshot' }),
    ).not.toBeInTheDocument();
  });

  it('creates a project and imports XML to load the saved snapshot', async () => {
    const user = userEvent.setup();
    let imported = false;
    const createProject = vi
      .fn<WorkspaceClient['createProject']>()
      .mockResolvedValue(a);
    const importXml = vi
      .fn<WorkspaceClient['importXml']>()
      .mockImplementation(async () => {
        imported = true;
      });
    render(
      <Workspace
        client={client({
          listProjects: async () => [],
          createProject,
          importXml,
          loadProjectSnapshot: async () => (imported ? snapshot : null),
        })}
        onDemo={() => {}}
      />,
    );
    await user.type(
      await screen.findByLabelText('Nombre del nuevo proyecto'),
      'Proyecto Alfa',
    );
    await user.click(screen.getByRole('button', { name: 'Crear proyecto' }));
    expect(
      await screen.findByRole('heading', { name: 'Sin snapshot disponible' }),
    ).toBeInTheDocument();
    await user.upload(
      screen.getByLabelText('Archivo XML Nmap'),
      new File(['<nmaprun/>'], 'lab.xml', { type: 'application/xml' }),
    );
    await user.click(screen.getByRole('button', { name: 'Importar XML' }));
    expect(
      await screen.findByRole('heading', { name: 'Resumen del snapshot' }),
    ).toBeInTheDocument();
    expect(createProject).toHaveBeenCalledWith(
      'Proyecto Alfa',
      expect.any(AbortSignal),
    );
    expect(importXml).toHaveBeenCalledWith(
      a.project.id,
      expect.any(File),
      expect.any(AbortSignal),
    );
    expect(screen.getByText('Importación guardada.')).toBeInTheDocument();
  });

  it('keeps previous results when an import is rejected and displays a safe error', async () => {
    const user = userEvent.setup();
    render(
      <Workspace
        client={client({
          importXml: async () => {
            throw new ApiError(422);
          },
        })}
        onDemo={() => {}}
      />,
    );
    await screen.findByRole('heading', { name: 'Resumen del snapshot' });
    await user.upload(
      screen.getByLabelText('Archivo XML Nmap'),
      new File(['<nmaprun/>'], 'outside.xml', { type: 'application/xml' }),
    );
    await user.click(screen.getByRole('button', { name: 'Importar XML' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'fuera del scope',
    );
    expect(
      screen.getByRole('heading', { name: 'Resumen del snapshot' }),
    ).toBeInTheDocument();
    expect(screen.queryByText('Importación guardada.')).not.toBeInTheDocument();
  });

  it('clears project results when the session expires', async () => {
    const user = userEvent.setup();
    render(
      <Workspace
        client={client({
          importXml: async () => {
            throw new ApiError(401);
          },
        })}
        onDemo={() => {}}
      />,
    );
    await screen.findByRole('heading', { name: 'Resumen del snapshot' });
    await user.upload(
      screen.getByLabelText('Archivo XML Nmap'),
      new File(['<nmaprun/>'], 'lab.xml', { type: 'application/xml' }),
    );
    await user.click(screen.getByRole('button', { name: 'Importar XML' }));
    expect(
      await screen.findByRole('button', { name: 'Iniciar sesión con GitHub' }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole('heading', { name: 'Proyecto Alfa' }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('heading', { name: 'Resumen del snapshot' }),
    ).not.toBeInTheDocument();
  });

  it('ignores an earlier project response after selecting another project', async () => {
    const user = userEvent.setup();
    let resolveA!: (value: typeof snapshot) => void;
    let oldSignal: AbortSignal | undefined;
    const pending = new Promise<typeof snapshot>((resolve) => {
      resolveA = resolve;
    });
    const load = vi
      .fn<WorkspaceClient['loadProjectSnapshot']>()
      .mockImplementation((id, signal) => {
        if (id === a.project.id) {
          oldSignal = signal;
          return pending;
        }
        return Promise.resolve(null);
      });
    render(
      <Workspace
        client={client({
          listProjects: async () => [a, b],
          loadProjectSnapshot: load,
        })}
        onDemo={() => {}}
      />,
    );
    await waitFor(() => expect(load).toHaveBeenCalled());
    await user.selectOptions(
      screen.getByLabelText('Proyecto activo'),
      b.project.id,
    );
    expect(
      await screen.findByRole('heading', { name: 'Proyecto Beta' }),
    ).toBeInTheDocument();
    expect(oldSignal?.aborted).toBe(true);
    await act(async () => {
      resolveA(snapshot);
      await pending;
    });
    expect(
      screen.queryByRole('heading', { name: 'Resumen del snapshot' }),
    ).not.toBeInTheDocument();
    expect(
      await screen.findByRole('heading', { name: 'Sin snapshot disponible' }),
    ).toBeInTheDocument();
  });

  it('aborts the XML request when the project view unmounts', async () => {
    const user = userEvent.setup();
    let signal: AbortSignal | undefined;
    const view = render(
      <Workspace
        client={client({
          importXml: async (_id, _file, received) => {
            signal = received;
            await new Promise<void>(() => {});
          },
        })}
        onDemo={() => {}}
      />,
    );
    await screen.findByRole('heading', { name: 'Resumen del snapshot' });
    await user.upload(
      screen.getByLabelText('Archivo XML Nmap'),
      new File(['<nmaprun/>'], 'lab.xml', { type: 'application/xml' }),
    );
    await user.click(screen.getByRole('button', { name: 'Importar XML' }));
    view.unmount();
    expect(signal?.aborted).toBe(true);
  });

  it('does not import an empty XML and clears the workspace at logout', async () => {
    const user = userEvent.setup(),
      importXml = vi.fn<WorkspaceClient['importXml']>();
    render(<Workspace client={client({ importXml })} onDemo={() => {}} />);
    await screen.findByRole('heading', { name: 'Resumen del snapshot' });
    await user.upload(
      screen.getByLabelText('Archivo XML Nmap'),
      new File([], 'empty.xml', { type: 'application/xml' }),
    );
    await user.click(screen.getByRole('button', { name: 'Importar XML' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('2 MiB');
    expect(importXml).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Cerrar sesión' }));
    expect(
      await screen.findByRole('button', { name: 'Iniciar sesión con GitHub' }),
    ).toBeInTheDocument();
    expect(screen.queryByLabelText('Proyecto activo')).not.toBeInTheDocument();
  });
});
