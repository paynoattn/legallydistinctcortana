import { globSync, readFileSync } from 'node:fs';
import { basename, join } from 'node:path';
import { gzipSync } from 'node:zlib';

const BYTES_PER_KB = 1024;
const SIZE_COLUMN_WIDTH = 7;
const ASSETS = join('dist', 'assets');
const budgetKb = Number(process.env['BUNDLE_BUDGET_KB'] ?? '80');

const WINDOWS: readonly { readonly name: string; readonly feature: string }[] = [
  { name: 'main', feature: 'tasks' },
  { name: 'read-aloud', feature: 'read-aloud' },
  { name: 'settings', feature: 'settings' },
  { name: 'overlay', feature: 'overlay' },
];

const STATIC_IMPORT = /(?:from|import)\s*["']\.\/([^"']+\.js)["']/g;

function fail(message: string): never {
  process.stderr.write(`bundle-budget: ${message}\n`);
  process.exit(1);
}

const chunks = globSync(join(ASSETS, '*.js')).map((file) => basename(file));
if (chunks.length === 0) {
  fail('no dist/assets/*.js found. Run `make build` first.');
}

const entry = /src="\/assets\/([^"]+\.js)"/.exec(readFileSync(join('dist', 'index.html'), 'utf8'));
const entryChunk = entry?.[1] ?? fail('dist/index.html loads no script from /assets.');

function sourcesOf(chunk: string): readonly string[] {
  const map: unknown = JSON.parse(readFileSync(join(ASSETS, `${chunk}.map`), 'utf8'));
  if (typeof map !== 'object' || map === null || !('sources' in map)) {
    return [];
  }
  const { sources } = map;
  return Array.isArray(sources)
    ? sources.filter((source: unknown): source is string => typeof source === 'string')
    : [];
}

function importsOf(chunk: string): readonly string[] {
  const code = readFileSync(join(ASSETS, chunk), 'utf8');
  return [...code.matchAll(STATIC_IMPORT)].flatMap(([, chunk]) =>
    chunk === undefined ? [] : [chunk],
  );
}

function loadedWith(starts: readonly string[]): ReadonlySet<string> {
  const loaded = new Set<string>();
  const pending = [...starts];
  for (let chunk = pending.pop(); chunk !== undefined; chunk = pending.pop()) {
    if (!loaded.has(chunk)) {
      loaded.add(chunk);
      pending.push(...importsOf(chunk));
    }
  }
  return loaded;
}

function kbOf(chunk: string): number {
  return gzipSync(readFileSync(join(ASSETS, chunk))).length / BYTES_PER_KB;
}

function line(kb: number, label: string): string {
  return `${kb.toFixed(1).padStart(SIZE_COLUMN_WIDTH)} KB gzip  ${label}\n`;
}

const over = WINDOWS.flatMap(({ name, feature }) => {
  const own = chunks.filter((chunk) =>
    sourcesOf(chunk).some((source) => source.includes(`/src/${feature}/`)),
  );
  const loaded = [...loadedWith([entryChunk, ...own])].sort((a, b) => a.localeCompare(b));
  const sizes = loaded.map((chunk) => ({ chunk, kb: kbOf(chunk) }));
  const totalKb = sizes.reduce((sum, { kb }) => sum + kb, 0);
  process.stdout.write(`${name} window (?view=${name === 'main' ? '' : name}):\n`);
  for (const { chunk, kb } of sizes) {
    process.stdout.write(line(kb, chunk));
  }
  process.stdout.write(line(totalKb, `total (BUNDLE_BUDGET_KB=${String(budgetKb)})`));
  return totalKb > budgetKb ? [`${name} is ${totalKb.toFixed(1)} KB`] : [];
});

if (over.length > 0) {
  fail(
    `JavaScript a window loads is over the ${String(budgetKb)} KB gzip budget: ${over.join(', ')}. ` +
      'Open dist/assets/*.js.map in a source-map explorer to find what grew, then remove it, ' +
      'replace it with something smaller, or load it lazily with a dynamic import.',
  );
}
