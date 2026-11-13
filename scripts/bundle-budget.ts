import { globSync, readFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';

const BYTES_PER_KB = 1024;
const SIZE_COLUMN_WIDTH = 7;
const budgetKb = Number(process.env['BUNDLE_BUDGET_KB'] ?? '80');

function fail(message: string): never {
  process.stderr.write(`bundle-budget: ${message}\n`);
  process.exit(1);
}

const files = globSync('dist/assets/*.js');
if (files.length === 0) {
  fail('no dist/assets/*.js found. Run `make build` first.');
}

const sizes = files.map((file) => ({
  file,
  kb: gzipSync(readFileSync(file)).length / BYTES_PER_KB,
}));
const totalKb = sizes.reduce((sum, { kb }) => sum + kb, 0);

for (const { file, kb } of sizes) {
  process.stdout.write(`${kb.toFixed(1).padStart(SIZE_COLUMN_WIDTH)} KB gzip  ${file}\n`);
}
process.stdout.write(
  `${totalKb.toFixed(1).padStart(SIZE_COLUMN_WIDTH)} KB gzip  total (BUNDLE_BUDGET_KB=${String(budgetKb)})\n`,
);

if (totalKb > budgetKb) {
  fail(
    `JavaScript is ${totalKb.toFixed(1)} KB gzip, over the ${String(budgetKb)} KB budget. ` +
      'Open dist/assets/*.js.map in a source-map explorer to find what grew, then remove it, ' +
      'replace it with something smaller, or load it lazily with React.lazy.',
  );
}
