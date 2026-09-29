import { readFileSync, readdirSync } from 'node:fs';
import { resolve, dirname, relative } from 'node:path';
import assert from 'node:assert/strict';
import ts from 'typescript';

const root = process.cwd();
function walk(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory()
      ? walk(resolve(directory, entry.name))
      : [resolve(directory, entry.name)],
  );
}
let checked = 0;
for (const layer of ['domain', 'application']) {
  for (const file of walk(`packages/${layer}`)) {
    if (!file.endsWith('.ts')) continue;
    const ast = ts.createSourceFile(
      file,
      readFileSync(file, 'utf8'),
      ts.ScriptTarget.Latest,
      true,
    );
    function visit(node) {
      const specifier =
        ts.isImportDeclaration(node) || ts.isExportDeclaration(node)
          ? node.moduleSpecifier
          : undefined;
      if (specifier && ts.isStringLiteral(specifier)) {
        const value = specifier.text;
        assert.ok(
          value.startsWith('.'),
          `${relative(root, file)} imports infrastructure/external module ${value}`,
        );
        const target = resolve(dirname(file), value);
        const allowed = [
          resolve(`packages/${layer}`),
          resolve('packages/domain'),
        ];
        assert.ok(
          allowed.some((path) => !relative(path, target).startsWith('..')),
          `${relative(root, file)} violates layer dependency: ${value}`,
        );
      }
      if (ts.isCallExpression(node)) {
        assert.ok(
          node.expression.kind !== ts.SyntaxKind.ImportKeyword,
          `Dynamic infrastructure import in ${file}`,
        );
        if (ts.isIdentifier(node.expression))
          assert.ok(
            !['require', 'eval', 'fetch'].includes(node.expression.text),
            `Forbidden runtime IO/eval in ${file}`,
          );
      }
      ts.forEachChild(node, visit);
    }
    visit(ast);
    checked++;
  }
}
console.log(
  `Validated dependency direction for ${checked} domain/application modules.`,
);
