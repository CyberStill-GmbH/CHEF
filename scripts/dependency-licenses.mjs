import { readFileSync } from 'node:fs';
const lock = JSON.parse(readFileSync('package-lock.json', 'utf8'));
const inventory = Object.entries(lock.packages)
  .filter(([path]) => path)
  .map(([path, item]) => {
    let license = item.license;
    let licenseSource = 'package-lock.json metadata';
    if (!license && path === 'node_modules/khroma') {
      const bundled = readFileSync(`${path}/license`, 'utf8');
      if (bundled.startsWith('The MIT License (MIT)')) {
        license = 'MIT';
        licenseSource = `${path}/license`;
      }
    }
    return {
      name: path.split('node_modules/').at(-1),
      version: item.version,
      license: license ?? 'REVIEW_REQUIRED',
      licenseSource,
      dev: Boolean(item.dev),
    };
  })
  .sort((a, b) => a.name.localeCompare(b.name));
console.log(
  JSON.stringify(
    {
      generatedFrom: 'package-lock.json',
      purpose:
        'Inventory; metadata does not substitute for reviewing applicable license texts and notices.',
      packages: inventory,
    },
    null,
    2,
  ),
);
