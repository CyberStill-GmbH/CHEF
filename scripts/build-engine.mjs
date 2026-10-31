import { execFileSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

mkdirSync('artifacts/bin', { recursive: true });
const executable = resolve(
  `artifacts/bin/correlate${process.platform === 'win32' ? '.exe' : ''}`,
);
execFileSync(
  'go',
  ['build', '-trimpath', '-o', executable, './cmd/correlate'],
  {
    cwd: 'engine',
    stdio: 'inherit',
    env: { ...process.env, CGO_ENABLED: '0' },
  },
);
console.log('Built local correlation engine');
