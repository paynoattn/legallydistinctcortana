const LAYER = '^src/([^/]+)/(domain|application|infrastructure|ui)/';
const COMPOSITION_ROOT = '^src/main\\.tsx$';
const SPEC = '\\.spec\\.tsx?$';

/** @type {import('dependency-cruiser').IConfiguration} */
module.exports = {
  forbidden: [
    {
      name: 'no-circular',
      severity: 'error',
      comment: 'Cycles make modules impossible to understand or change in isolation.',
      from: {},
      to: { circular: true },
    },
    {
      name: 'no-orphans',
      severity: 'error',
      comment: 'Nothing imports this file. Delete it or wire it in.',
      from: {
        orphan: true,
        pathNot: [SPEC, COMPOSITION_ROOT, '\\.d\\.ts$', '^(features|qa|scripts)/', '^[^/]+$'],
      },
      to: {},
    },
    {
      name: 'domain-is-pure',
      severity: 'error',
      comment:
        'domain imports only its own feature domain: no React, npm packages, browser APIs wrappers, or other layers.',
      from: { path: '^src/([^/]+)/domain/', pathNot: SPEC },
      to: { pathNot: '^src/$1/domain/' },
    },
    {
      name: 'application-depends-on-domain-only',
      severity: 'error',
      comment:
        'application may import its own domain and application only (no React). Describe external needs as a port interface.',
      from: { path: '^src/([^/]+)/application/', pathNot: SPEC },
      to: { pathNot: '^src/$1/(domain|application)/' },
    },
    {
      name: 'ui-not-to-infrastructure',
      severity: 'error',
      comment:
        'Components talk to application (the store), never to adapters. Inject through props.',
      from: { path: '^src/[^/]+/ui/', pathNot: SPEC },
      to: { path: '^src/[^/]+/infrastructure/' },
    },
    {
      name: 'infrastructure-not-to-ui',
      severity: 'error',
      comment: 'infrastructure implements application ports; it does not know about components.',
      from: { path: '^src/[^/]+/infrastructure/' },
      to: { path: '^src/[^/]+/ui/' },
    },
    {
      name: 'only-react-in-ui',
      severity: 'error',
      comment:
        'React belongs to ui and the composition root. Keep domain, application, and infrastructure framework-free.',
      from: { path: '^src/', pathNot: ['^src/[^/]+/ui/', COMPOSITION_ROOT] },
      to: { path: '(^|/)node_modules/(react|react-dom)/' },
    },
    {
      name: 'only-composition-root-imports-infrastructure',
      severity: 'error',
      comment:
        'Only src/main.tsx (and tests, which assemble real adapters) wire adapters into ports.',
      from: { pathNot: [COMPOSITION_ROOT, SPEC, '^src/[^/]+/infrastructure/'] },
      to: { path: '^src/[^/]+/infrastructure/' },
    },
    {
      name: 'feature-public-entry-only',
      severity: 'error',
      comment: 'Import another feature only through its index.ts, never its internals.',
      from: { path: '^src/([^/]+)/' },
      to: {
        path: '^src/[^/]+/',
        pathNot: ['^src/$1/', '^src/shared/', '^src/[^/]+/index\\.tsx?$'],
      },
    },
    {
      name: 'shared-is-feature-agnostic',
      severity: 'error',
      comment: 'src/shared holds cross-cutting helpers and must not depend on any feature.',
      from: { path: '^src/shared/' },
      to: { path: '^src/(?!shared/)' },
    },
    {
      name: 'layers-not-to-composition-root',
      severity: 'error',
      comment: 'Layers do not import main.tsx; the composition root depends on them.',
      from: { path: LAYER },
      to: { path: COMPOSITION_ROOT },
    },
    {
      name: 'no-node-builtins-in-the-browser',
      severity: 'error',
      comment: 'src/ ships to the browser. Node built-ins either break the bundle or bloat it.',
      from: { path: '^src/' },
      to: { dependencyTypes: ['core'] },
    },
    {
      name: 'not-to-spec',
      severity: 'error',
      comment: 'Production code must not import test files.',
      from: { pathNot: SPEC },
      to: { path: SPEC },
    },
    {
      name: 'not-to-dev-dep',
      severity: 'error',
      comment: 'Production code must not depend on devDependencies.',
      from: { path: '^src/', pathNot: SPEC },
      to: { dependencyTypes: ['npm-dev'], dependencyTypesNot: ['type-only'] },
    },
    {
      name: 'qa-is-black-box',
      severity: 'error',
      comment: 'QA scripts drive the built app in a browser only.',
      from: { path: '^qa/' },
      to: { path: '^(src|features)/' },
    },
    {
      name: 'acceptance-is-black-box',
      severity: 'error',
      comment: 'Step definitions drive the running app in a browser and never import its code.',
      from: { path: '^features/' },
      to: { path: '^src/' },
    },
    {
      name: 'not-to-unresolvable',
      severity: 'error',
      comment: 'This import does not resolve. Fix the path or add the package.',
      from: {},
      to: { couldNotResolve: true },
    },
  ],
  options: {
    doNotFollow: { path: 'node_modules' },
    tsPreCompilationDeps: true,
    tsConfig: { fileName: 'src/tsconfig.json' },
    enhancedResolveOptions: {
      exportsFields: ['exports'],
      conditionNames: ['import', 'require', 'node', 'default', 'types'],
      mainFields: ['module', 'main', 'types', 'typings'],
      extensions: ['.ts', '.tsx', '.js', '.mjs', '.cjs', '.d.ts', '.json'],
    },
    reporterOptions: { text: { highlightFocused: true } },
  },
};
