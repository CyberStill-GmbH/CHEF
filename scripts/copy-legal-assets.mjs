import { copyFile, mkdir } from 'node:fs/promises';

const publicDirectory = new URL('../apps/web/public/', import.meta.url);

await mkdir(publicDirectory, { recursive: true });
await Promise.all(
  ['LICENSE', 'THIRD_PARTY_NOTICES.md'].map((filename) =>
    copyFile(
      new URL(`../${filename}`, import.meta.url),
      new URL(`../apps/web/public/${filename}`, import.meta.url),
    ),
  ),
);
