import { spawnSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { createReadStream, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

export function connectionEnvironment(databaseUrl) {
  let url;
  try {
    url = new URL(databaseUrl);
  } catch {
    throw new Error('DATABASE_URL no es una URL PostgreSQL válida');
  }
  if (
    !['postgres:', 'postgresql:'].includes(url.protocol) ||
    !url.hostname ||
    url.pathname.length < 2
  )
    throw new Error('DATABASE_URL requiere host y nombre de base PostgreSQL');
  const env = {
    PGHOST: url.hostname.replace(/^\[|\]$/g, ''),
    PGPORT: url.port || '5432',
    PGUSER: decodeURIComponent(url.username),
    PGDATABASE: decodeURIComponent(url.pathname.slice(1)),
    PGCONNECT_TIMEOUT: '10',
    PGAPPNAME: 'chef-db-operations',
  };
  const allowed = {
    sslmode: 'PGSSLMODE',
    connect_timeout: 'PGCONNECT_TIMEOUT',
  };
  for (const [key, value] of url.searchParams) {
    if (key === 'schema') continue;
    if (!Object.hasOwn(allowed, key))
      throw new Error(
        'DATABASE_URL contiene opciones no compatibles con las herramientas de backup',
      );
    env[allowed[key]] = value;
  }
  const password = decodeURIComponent(url.password);
  if (
    Object.values(env).some((value) => /[\r\n\0]/.test(value)) ||
    /[\r\n\0]/.test(password)
  )
    throw new Error('DATABASE_URL contiene caracteres de control');
  if (!env.PGUSER) throw new Error('DATABASE_URL requiere usuario');
  return { env, password };
}

export function sha256(file) {
  return new Promise((resolve, reject) => {
    const hash = createHash('sha256');
    createReadStream(file)
      .on('error', reject)
      .on('data', (chunk) => hash.update(chunk))
      .on('end', () => resolve(hash.digest('hex')));
  });
}

export function runDatabaseTool(
  tool,
  args,
  { input = 'ignore', output = 'ignore' } = {},
) {
  if (!['pg_dump', 'pg_restore'].includes(tool))
    throw new Error('Herramienta de base no permitida');
  const databaseUrl = process.env.DATABASE_URL;
  if (!databaseUrl)
    throw new Error('define DATABASE_URL (no la imprimas en logs)');
  const { env: connection, password } = connectionEnvironment(databaseUrl);
  let mode = process.env.CHEF_DB_TOOLS_MODE ?? 'auto';
  if (!['auto', 'native', 'docker'].includes(mode))
    throw new Error('CHEF_DB_TOOLS_MODE debe ser auto, native o docker');
  if (mode === 'auto')
    mode =
      spawnSync(tool, ['--version'], { stdio: 'ignore', windowsHide: true })
        .status === 0
        ? 'native'
        : 'docker';
  if (
    mode === 'docker' &&
    ['localhost', '127.0.0.1', '::1'].includes(connection.PGHOST)
  )
    connection.PGHOST = 'host.docker.internal';
  const credentials = mkdtempSync(join(tmpdir(), 'chef-pg-auth-'));
  let containerName;
  let failed = true;
  try {
    const passfile = join(credentials, 'pgpass');
    const escape = (value) => value.replace(/\\/g, '\\\\').replace(/:/g, '\\:');
    writeFileSync(
      passfile,
      [
        connection.PGHOST,
        connection.PGPORT,
        connection.PGDATABASE,
        connection.PGUSER,
        password,
      ]
        .map(escape)
        .join(':') + '\n',
      { mode: 0o600 },
    );
    const env = { ...process.env };
    for (const key of Object.keys(env))
      if (key.startsWith('PG') || key === 'DATABASE_URL') delete env[key];
    Object.assign(env, connection);
    let command = tool,
      commandArgs = args;
    if (mode === 'native') env.PGPASSFILE = passfile;
    else {
      if (credentials.includes(','))
        throw new Error('La ruta temporal de Docker no puede contener comas');
      env.PGPASSFILE = '/tmp/chef.pgpass';
      command = 'docker';
      containerName = `chef-db-tool-${randomUUID()}`;
      commandArgs = [
        'run',
        '--rm',
        '--name',
        containerName,
        '-i',
        '--add-host',
        'host.docker.internal:host-gateway',
      ];
      if (process.env.CHEF_DB_TOOLS_NETWORK)
        commandArgs.push('--network', process.env.CHEF_DB_TOOLS_NETWORK);
      commandArgs.push(
        '--mount',
        `type=bind,source=${credentials},target=/chef-auth,readonly`,
      );
      for (const key of [...Object.keys(connection), 'PGPASSFILE'])
        commandArgs.push('--env', key);
      // Copy to a private Linux file: Windows bind mounts do not preserve mode 0600.
      // Static shell text; connection values and arguments are never interpolated.
      commandArgs.push(
        '--entrypoint',
        'sh',
        process.env.CHEF_POSTGRES_IMAGE ?? 'postgres:16-alpine',
        '-c',
        'umask 077; cp /chef-auth/pgpass /tmp/chef.pgpass || exit 1; exec "$@"',
        'chef-db-tool',
        tool,
        ...args,
      );
    }
    const result = spawnSync(command, commandArgs, {
      env,
      stdio: [input, output, 'pipe'],
      windowsHide: true,
      timeout: 30 * 60 * 1000,
      maxBuffer: 1024 * 1024,
    });
    // Do not print libpq/Docker diagnostics: they can contain connection data.
    if (result.error || result.status !== 0)
      throw new Error(
        `${tool} falló en modo ${mode}; revisa conexión, permisos y versión del cliente (sin imprimir credenciales)`,
      );
    failed = false;
    return mode;
  } finally {
    if (failed && containerName)
      spawnSync('docker', ['rm', '--force', containerName], {
        stdio: 'ignore',
        windowsHide: true,
        timeout: 10_000,
      });
    // This exact directory was created by mkdtemp, never supplied by the user.
    rmSync(credentials, { recursive: true, force: true });
  }
}
