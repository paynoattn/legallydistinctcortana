import { globSync, readFileSync, statSync } from 'node:fs';
import { resolve } from 'node:path';
import { type AST, Linter, type Rule } from 'eslint';
import tseslint from 'typescript-eslint';
import { z } from 'zod/mini';

type SourceLocation = AST.SourceLocation;
type Position = SourceLocation['start'];

interface FunctionComplexity {
  readonly file: string;
  readonly name: string;
  readonly startLine: number;
  readonly endLine: number;
  readonly complexity: number;
}

interface CrapScore extends FunctionComplexity {
  readonly coverage: number;
  readonly crap: number;
}

const fileCoverage = z.object({
  statementMap: z.record(z.string(), z.object({ start: z.object({ line: z.number() }) })),
  s: z.record(z.string(), z.number()),
});
const coverageReport = z.record(z.string(), fileCoverage);

type FileCoverage = z.infer<typeof fileCoverage>;
type CoverageReport = Partial<z.infer<typeof coverageReport>>;

const COVERAGE_FILE = 'coverage/coverage-final.json';
const COMPLEXITY_MESSAGE = /^(.+) has a complexity of (\d+)\./;
const WORST_SHOWN = 10;
const PERCENT = 100;
const COLUMN_WIDTH = { crap: 6, complexity: 3, coverage: 5 };
const crapMax = Number(process.env['CRAP_MAX'] ?? '6');

function fail(message: string): never {
  process.stderr.write(`crap: ${message}\n`);
  process.exit(1);
}

const SOURCE_GLOB = 'src/**/*.{ts,tsx}';
const SPEC_FILE = /\.spec\.tsx?$/;

function sourceFiles(): string[] {
  return globSync(SOURCE_GLOB).filter((file) => !SPEC_FILE.test(file));
}

function functionRecorder(found: SourceLocation[]): Rule.RuleModule {
  const record = (node: Rule.Node): void => {
    if (node.loc) {
      found.push(node.loc);
    }
  };
  return {
    create: () => ({ 'FunctionDeclaration, FunctionExpression, ArrowFunctionExpression': record }),
  };
}

function isAtOrBefore(a: Position, b: Position): boolean {
  return a.line < b.line || (a.line === b.line && a.column <= b.column);
}

function enclosingFunctionEndLine(
  functions: readonly SourceLocation[],
  message: Linter.LintMessage,
): number {
  const headEnd = {
    line: message.endLine ?? message.line,
    column: (message.endColumn ?? message.column) - 1,
  };
  const enclosing = functions
    .filter((fn) => isAtOrBefore(fn.start, headEnd) && isAtOrBefore(headEnd, fn.end))
    .sort((a, b) => (isAtOrBefore(a.start, b.start) ? 1 : -1));
  return enclosing[0]?.end.line ?? headEnd.line;
}

function toFunctionComplexity(
  file: string,
  functions: readonly SourceLocation[],
  message: Linter.LintMessage,
): FunctionComplexity {
  const [, name = '?', complexity = '0'] = COMPLEXITY_MESSAGE.exec(message.message) ?? [];
  return {
    file,
    name,
    startLine: message.line,
    endLine: enclosingFunctionEndLine(functions, message),
    complexity: Number(complexity),
  };
}

function measureComplexity(file: string): FunctionComplexity[] {
  const functions: SourceLocation[] = [];
  const messages = new Linter().verify(
    readFileSync(file, 'utf8'),
    {
      files: ['**/*.ts', '**/*.tsx'],
      languageOptions: {
        parser: tseslint.parser,
        parserOptions: { ecmaFeatures: { jsx: file.endsWith('.tsx') } },
      },
      plugins: { crap: { rules: { functions: functionRecorder(functions) } } },
      rules: { complexity: ['error', 0], 'crap/functions': 'error' },
    },
    { filename: file },
  );
  const fatal = messages.find((message) => message.fatal === true);
  if (fatal !== undefined) {
    fail(`${file}:${String(fatal.line)}: ${fatal.message}`);
  }
  return messages.map((message) => toFunctionComplexity(file, functions, message));
}

function readCoverage(): CoverageReport {
  const reportTime = statSync(COVERAGE_FILE, { throwIfNoEntry: false })?.mtimeMs;
  if (reportTime === undefined) {
    fail(`${COVERAGE_FILE} not found. Run \`make coverage\` first.`);
  }
  const stale = globSync(SOURCE_GLOB).find((file) => statSync(file).mtimeMs > reportTime);
  if (stale !== undefined) {
    fail(`${COVERAGE_FILE} is older than ${stale}. Run \`make coverage\` first.`);
  }
  const report = coverageReport.safeParse(JSON.parse(readFileSync(COVERAGE_FILE, 'utf8')));
  if (!report.success) {
    fail(`${COVERAGE_FILE} is not an Istanbul coverage report: ${report.error.message}`);
  }
  return report.data;
}

function lineHits(coverage: FileCoverage, fn: FunctionComplexity): Map<number, number> {
  const hits = new Map<number, number>();
  for (const [id, location] of Object.entries(coverage.statementMap)) {
    const line = location.start.line;
    if (line >= fn.startLine && line <= fn.endLine) {
      hits.set(line, Math.max(hits.get(line) ?? 0, coverage.s[id] ?? 0));
    }
  }
  return hits;
}

function lineCoverage(coverage: FileCoverage | undefined, fn: FunctionComplexity): number {
  const hits = coverage === undefined ? new Map<number, number>() : lineHits(coverage, fn);
  if (hits.size === 0) {
    return 0;
  }
  return [...hits.values()].filter((count) => count > 0).length / hits.size;
}

function crapScore(complexity: number, coverage: number): number {
  const uncovered = 1 - coverage;
  return complexity * complexity * uncovered * uncovered * uncovered + complexity;
}

function score(report: CoverageReport, fn: FunctionComplexity): CrapScore {
  const coverage = lineCoverage(report[resolve(fn.file)], fn);
  return { ...fn, coverage, crap: crapScore(fn.complexity, coverage) };
}

function formatRow(fn: CrapScore): string {
  const crap = fn.crap.toFixed(1).padStart(COLUMN_WIDTH.crap);
  const complexity = String(fn.complexity).padStart(COLUMN_WIDTH.complexity);
  const coverage = `${(fn.coverage * PERCENT).toFixed(0)}%`.padStart(COLUMN_WIDTH.coverage);
  return `${crap} ${complexity} ${coverage}  ${fn.file}:${String(fn.startLine)}  ${fn.name}`;
}

function main(): void {
  const report = readCoverage();
  const scores = sourceFiles()
    .flatMap(measureComplexity)
    .map((fn) => score(report, fn))
    .sort((a, b) => b.crap - a.crap);
  const out = process.stdout;
  out.write(
    `Worst ${String(WORST_SHOWN)} of ${String(scores.length)} functions (CRAP_MAX=${String(crapMax)}):\n`,
  );
  out.write('  CRAP  CC   COV  LOCATION  FUNCTION\n');
  out.write(`${scores.slice(0, WORST_SHOWN).map(formatRow).join('\n')}\n`);
  const offenders = scores.filter((fn) => fn.crap > crapMax);
  if (offenders.length > 0) {
    fail(
      `${String(offenders.length)} function(s) above CRAP_MAX=${String(crapMax)}:\n${offenders.map(formatRow).join('\n')}\n` +
        'Cover the untested lines first, then reduce complexity (see the cleaner skill).',
    );
  }
}

main();
