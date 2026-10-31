import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
} from 'react';
import {
  ApiError,
  HttpWorkspaceClient,
  operationError,
} from './api/HttpWorkspaceClient';
import type { ProjectSummary, WorkspaceClient } from './api/WorkspaceClient';
import type { SnapshotDto } from './contracts/snapshot';
import { SnapshotView } from './SnapshotView';

const defaultClient = new HttpWorkspaceClient();
const MAX_XML_BYTES = 2 * 1024 * 1024;

function ProjectDetails({
  client,
  project,
  onExpired,
}: {
  client: WorkspaceClient;
  project: ProjectSummary;
  onExpired: () => void;
}) {
  const [snapshot, setSnapshot] = useState<SnapshotDto | null>(null);
  const [loading, setLoading] = useState(true);
  const [reload, setReload] = useState(0);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [importing, setImporting] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const importRequest = useRef<AbortController | null>(null);
  useEffect(
    () => () => {
      importRequest.current?.abort();
    },
    [],
  );

  useEffect(() => {
    const request = new AbortController();
    setLoading(true);
    setError('');
    setSnapshot(null);
    void client
      .loadProjectSnapshot(project.project.id, request.signal)
      .then((result) => {
        if (!request.signal.aborted) {
          setSnapshot(result);
          setLoading(false);
        }
      })
      .catch((failure) => {
        if (request.signal.aborted) return;
        if (failure instanceof ApiError && failure.status === 401) onExpired();
        else {
          setError(operationError(failure));
          setLoading(false);
        }
      });
    return () => request.abort();
  }, [client, project.project.id, onExpired, reload]);

  const snapshotClient = useMemo(
    () => ({
      loadSnapshot: async () => {
        if (!snapshot) throw new Error('Snapshot unavailable');
        return snapshot;
      },
    }),
    [snapshot],
  );

  async function importFile(event: FormEvent) {
    event.preventDefault();
    const file = input.current?.files?.[0];
    if (!file) {
      setError('Selecciona un archivo XML Nmap.');
      return;
    }
    if (file.size === 0 || file.size > MAX_XML_BYTES) {
      setError('El XML debe contener datos y no superar 2 MiB.');
      return;
    }
    const request = new AbortController();
    importRequest.current = request;
    setImporting(true);
    setError('');
    setNotice('');
    try {
      await client.importXml(project.project.id, file, request.signal);
      if (request.signal.aborted) return;
      if (input.current) input.current.value = '';
      setNotice('Importación guardada.');
      setReload((value) => value + 1);
    } catch (failure) {
      if (request.signal.aborted) return;
      if (failure instanceof ApiError && failure.status === 401) onExpired();
      else setError(operationError(failure));
    } finally {
      if (!request.signal.aborted) setImporting(false);
    }
  }

  return (
    <>
      <section aria-labelledby="active-project-title">
        <h2 id="active-project-title">{project.project.name}</h2>
        <form
          onSubmit={(event) => {
            void importFile(event);
          }}
          aria-label="Importar XML Nmap"
        >
          <label htmlFor="nmap-file">Archivo XML Nmap</label>
          <input
            ref={input}
            id="nmap-file"
            name="xml"
            type="file"
            accept=".xml,application/xml,text/xml"
            disabled={importing}
            aria-describedby="import-help"
          />
          <p id="import-help">
            Hasta 2 MiB. Solo servicios dentro del scope autorizado del
            servidor.
          </p>
          <button type="submit" disabled={importing}>
            {importing ? 'Importando…' : 'Importar XML'}
          </button>
        </form>
        <button
          type="button"
          onClick={() => setReload((value) => value + 1)}
          disabled={loading || importing}
        >
          Actualizar resultados
        </button>
        {notice && <p role="status">{notice}</p>}
        {error && <p role="alert">{error}</p>}
      </section>
      {loading ? (
        <p role="status">Cargando resultados…</p>
      ) : snapshot ? (
        <SnapshotView client={snapshotClient} />
      ) : (
        !error && (
          <section>
            <h2>Sin snapshot disponible</h2>
            <p>
              Importa un XML autorizado para consultar los resultados de este
              proyecto.
            </p>
          </section>
        )
      )}
    </>
  );
}

function Projects({
  client,
  onExpired,
}: {
  client: WorkspaceClient;
  onExpired: () => void;
}) {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [selectedId, setSelectedId] = useState('');
  const [loading, setLoading] = useState(true);
  const [reload, setReload] = useState(0);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState('');
  const [name, setName] = useState('');
  const createRequest = useRef<AbortController | null>(null);
  useEffect(
    () => () => {
      createRequest.current?.abort();
    },
    [],
  );
  const fail = useCallback(
    (failure: unknown) => {
      if (failure instanceof ApiError && failure.status === 401) onExpired();
      else setError(operationError(failure));
    },
    [onExpired],
  );
  useEffect(() => {
    const request = new AbortController();
    setLoading(true);
    setError('');
    void client
      .listProjects(request.signal)
      .then((items) => {
        if (request.signal.aborted) return;
        setProjects(items);
        setSelectedId((current) =>
          items.some((item) => item.project.id === current)
            ? current
            : (items[0]?.project.id ?? ''),
        );
        setLoading(false);
      })
      .catch((failure) => {
        if (!request.signal.aborted) {
          fail(failure);
          setLoading(false);
        }
      });
    return () => request.abort();
  }, [client, fail, reload]);

  async function create(event: FormEvent) {
    event.preventDefault();
    if (name.trim().length < 2) {
      setError('Escribe un nombre de al menos dos caracteres.');
      return;
    }
    const request = new AbortController();
    createRequest.current = request;
    setCreating(true);
    setError('');
    try {
      const item = await client.createProject(name.trim(), request.signal);
      if (request.signal.aborted) return;
      setProjects((current) => [...current, item]);
      setSelectedId(item.project.id);
      setName('');
    } catch (failure) {
      if (!request.signal.aborted) fail(failure);
    } finally {
      if (!request.signal.aborted) setCreating(false);
    }
  }
  const selected = projects.find((item) => item.project.id === selectedId);
  return (
    <>
      <section aria-labelledby="projects-title">
        <h2 id="projects-title">Tus proyectos</h2>
        {loading ? (
          <p role="status">Cargando proyectos…</p>
        ) : (
          <>
            {projects.length === 0 ? (
              <p>Todavía no tienes proyectos.</p>
            ) : (
              <>
                <label htmlFor="project-select">Proyecto activo</label>
                <select
                  id="project-select"
                  value={selectedId}
                  onChange={(event) => setSelectedId(event.target.value)}
                >
                  {projects.map((item) => (
                    <option key={item.project.id} value={item.project.id}>
                      {item.project.name}
                    </option>
                  ))}
                </select>
              </>
            )}
            <form
              onSubmit={(event) => {
                void create(event);
              }}
              aria-label="Crear proyecto"
            >
              <label htmlFor="project-name">Nombre del nuevo proyecto</label>
              <input
                id="project-name"
                value={name}
                onChange={(event) => setName(event.target.value)}
                maxLength={120}
                required
                minLength={2}
                disabled={creating}
              />
              <button type="submit" disabled={creating}>
                {creating ? 'Creando…' : 'Crear proyecto'}
              </button>
            </form>
          </>
        )}
        {error && <p role="alert">{error}</p>}
        <button
          type="button"
          disabled={loading || creating}
          onClick={() => setReload((value) => value + 1)}
        >
          Actualizar proyectos
        </button>
      </section>
      {selected && (
        <ProjectDetails
          key={selected.project.id}
          client={client}
          project={selected}
          onExpired={onExpired}
        />
      )}
    </>
  );
}

export function Workspace({
  client = defaultClient,
  onDemo,
  navigate = (url) => window.location.assign(url),
}: {
  client?: WorkspaceClient;
  onDemo: () => void;
  navigate?: (url: string) => void;
}) {
  const [auth, setAuth] = useState<
    'loading' | 'signed-out' | 'signed-in' | 'error'
  >('loading');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [reload, setReload] = useState(0);
  const expired = useCallback(() => {
    setAuth('signed-out');
    setError('La sesión ha caducado. Inicia sesión de nuevo.');
  }, []);
  useEffect(() => {
    const request = new AbortController();
    const callbackFailed =
      new URLSearchParams(window.location.search).get('auth') === 'failed';
    if (callbackFailed) {
      setError('No se pudo iniciar sesión con GitHub. Vuelve a intentarlo.');
      window.history.replaceState(null, '', window.location.pathname);
    }
    setAuth('loading');
    void client
      .session(request.signal)
      .then((session) => {
        if (!request.signal.aborted)
          setAuth(session ? 'signed-in' : 'signed-out');
      })
      .catch((failure) => {
        if (!request.signal.aborted) {
          setAuth('error');
          setError(operationError(failure));
        }
      });
    return () => request.abort();
  }, [client, reload]);

  async function signIn() {
    setBusy(true);
    setError('');
    try {
      navigate(await client.startSignIn());
    } catch (failure) {
      setError(operationError(failure));
      setBusy(false);
    }
  }
  async function logout() {
    setBusy(true);
    setError('');
    try {
      await client.logout();
      setAuth('signed-out');
    } catch (failure) {
      setError(operationError(failure));
    } finally {
      setBusy(false);
    }
  }
  return (
    <main>
      <header className="app-header">
        <div>
          <h1>CHEF</h1>
          <p>Proyectos y evidencia de superficie autorizada</p>
        </div>
        {auth === 'signed-in' && (
          <button
            disabled={busy}
            onClick={() => {
              void logout();
            }}
          >
            Cerrar sesión
          </button>
        )}
      </header>
      {auth === 'loading' && <p role="status">Comprobando sesión…</p>}
      {error && <p role="alert">{error}</p>}
      {auth === 'error' && (
        <button
          onClick={() => {
            setError('');
            setReload((value) => value + 1);
          }}
        >
          Reintentar conexión
        </button>
      )}
      {auth === 'signed-out' && (
        <section>
          <h2>Abre tu espacio de trabajo</h2>
          <p>
            Inicia sesión para consultar tus proyectos e importar evidencia.
          </p>
          <button
            disabled={busy}
            onClick={() => {
              void signIn();
            }}
          >
            {busy ? 'Conectando…' : 'Iniciar sesión con GitHub'}
          </button>
        </section>
      )}
      {auth === 'signed-in' && <Projects client={client} onExpired={expired} />}
      {auth !== 'signed-in' && (
        <button disabled={busy} onClick={onDemo}>
          Ver demo con datos sintéticos
        </button>
      )}
    </main>
  );
}
