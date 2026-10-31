import { useState } from 'react';
import type { ApiClient } from './api/ApiClient';
import { FixtureApiClient } from './api/FixtureApiClient';
import { SnapshotView } from './SnapshotView';
import { Workspace } from './Workspace';

const fixtureClient = new FixtureApiClient();
export function App({ client }: { client?: ApiClient }) {
  const [demo, setDemo] = useState(false);
  if (client)
    return (
      <main>
        <h1>CHEF</h1>
        <SnapshotView client={client} />
      </main>
    );
  if (demo)
    return (
      <main>
        <header>
          <h1>CHEF — demo</h1>
          <p>Datos sintéticos de laboratorio. No son proyectos de tu cuenta.</p>
          <button onClick={() => setDemo(false)}>Volver a mis proyectos</button>
        </header>
        <SnapshotView client={fixtureClient} />
      </main>
    );
  return <Workspace onDemo={() => setDemo(true)} />;
}
