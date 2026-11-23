# TypeScript / React style

## Code

- Names state intent; if a line needs a comment to be understood, rename or extract instead.
- Functions do one thing, stay under ~40 lines, take at most 3 positional args, otherwise one options object.
- Early return over nesting. No nested ternaries.
- `const` by default; no `let` unless reassigned; no mutation of arguments or props.
- No `any`. Unknown input is `unknown` and narrowed with a zod schema or type guard.
- No `as` casts and no `!` non-null assertions; fix the type instead. `as const` and `satisfies` are fine.
- No `enum`. Use string-literal unions (`type Role = "owner" | "member"`).
- `type` for all shapes; `interface` only when declaration merging is needed.
- No default exports (exceptions: config files, lazy-loaded components if required). Named exports only.
- Use `import type` / inline `type` specifiers for types (enforced by `verbatimModuleSyntax`).
- Booleans read as predicates: `isOpen`, `hasChanges`, `canRestore`.
- Event handler props are `onX`, handler functions are `handleX`.
- No magic numbers or strings repeated twice: name them in the slice's `config` segment.
- No dead code, no commented-out code, no `TODO` left in main.
- Prefer `Array.prototype` methods and `Object.entries` over loops; use `for...of` when you need `await` or early exit.
- `async`/`await` only; no `.then` chains. Every promise is awaited, returned, or explicitly `void`ed (lint-enforced).

## Naming and files

- Files and folders: `kebab-case` (`file-viewer.tsx`, `use-debounced-value.ts`).
- One component per file, file named after the component (`PageTitle` in `page-title.tsx`).
- Components `PascalCase`, hooks `useX`, types `PascalCase`, constants `camelCase` (`SCREAMING_CASE` only for true module-level constants like env keys).
- Test files sit next to the code: `x.ts` and `x.test.ts`. E2E specs live in `e2e/`.
- Alias `@/*` maps to `src/*`; use it across slices, relative imports inside one slice.

## Feature-Sliced Design

Source: https://feature-sliced.design/docs/reference/layers, https://feature-sliced.design/docs/reference/public-api

- Layers, top to bottom: `app`, `pages`, `widgets`, `features`, `entities`, `shared`. No other layers (no `processes`).
- A module imports only from layers strictly below. Never upward, never sideways between slices of one layer.
- `app` and `shared` have no slices; their segments may import each other.
- Segments are named by purpose: `ui`, `api`, `model`, `lib`, `config`. Never `components`, `hooks`, `types`, `utils`.
- Every slice exposes `index.ts` as its only public API; outside code imports the slice, never its internals.
- `index.ts` uses named re-exports, never `export *`.
- Inside a slice, import relatively; across slices, import via `@/layer/slice`.
- `shared/ui` and `shared/lib` get one folder plus `index.ts` per unit (`@/shared/ui/button`); no layer-wide barrel.
- Cross-entity links go through `entities/a/@x/b.ts`, rarely; prefer composing in `features` or `pages`.
- Put code in the lowest layer that does not need to know about the domain. Do not pre-create slices; extract on the second use.
- `pages` = one slice per screen, composes widgets and features and owns that screen's search-param schema.
- `app` owns providers, router, route files (thin: route definition + loader + page import), global styles.

```
src/
  app/        routes/, providers/, styles/, main.tsx
  pages/      home/ files/ settings/   (each: ui/ model/ index.ts)
  widgets/    sidebar-tree/ diff-view/
  features/   edit-page/ restore-version/
  entities/   page/ version/           (each: api/ model/ ui/ index.ts)
  shared/     api/ ui/ lib/ config/
```

## React

Source: https://react.dev/reference/rules, https://react.dev/learn/you-might-not-need-an-effect

- Function components only. Components and hooks are pure: no side effects or mutation during render.
- Call hooks at the top level, never in conditions, loops, or after early returns (lint-enforced).
- Props: `type XProps = { ... }` declared above the component, destructured in the signature; no `React.FC`.
- Do not store derived values in state; compute them during render.
- Do not use an Effect for what an event handler can do; user actions run in handlers.
- Reset state with `key`, not an Effect.
- Use an Effect only to sync with something outside React (DOM APIs, CodeMirror, subscriptions); always return cleanup.
- Subscribe to external stores with `useSyncExternalStore`.
- Effect dependency arrays are complete; fix the code, never silence `exhaustive-deps`.
- `key` is a stable id, never an array index for reorderable lists.
- Memoize (`useMemo`, `useCallback`, `memo`) only after measuring; not by default.
- Pass children and composition over boolean-prop explosions; at most 2 boolean props per component.
- Lazy-load heavy widgets (CodeMirror, diff view) with `React.lazy` at the page boundary.
- CodeMirror lives in one `shared/ui` wrapper component: create the view in an Effect, destroy in cleanup, content changes go through transactions, not re-creation.

## State and data

Source: https://tanstack.com/query/latest/docs/framework/react/guides/query-options, https://tanstack.com/router/latest/docs/framework/react/guide/search-params

- Server state lives only in TanStack Query. Never copy query data into `useState` or a store.
- Shareable UI state (filters, selected item, tab, sort) lives in Router search params, validated with a zod schema via `validateSearch`.
- Local UI state (open, hover, draft input) lives in `useState` in the lowest component that needs it.
- No global client store by default; add one only for state that is neither server data nor URL-addressable and is shared across distant components.
- Define every query with a `queryOptions` factory in the entity's `api` segment; components never write `queryKey` inline.
- Query keys are arrays from general to specific: `["page", id]`, `["page", id, "versions"]`. Invalidate by prefix.
- Mutations live in the feature's `api` segment and invalidate the keys they affect in `onSuccess`.
- Route loaders prefetch with `queryClient.ensureQueryData(options)`; components read with `useSuspenseQuery`.
- Do not set `staleTime` per query without a reason; set the default once in `app`.
- Search-param defaults live in the zod schema (`.default`), not in components.

## API layer

Source: https://openapi-ts.dev/openapi-fetch/

- Types are generated from the server's OpenAPI by `openapi-typescript` into `shared/api/schema.d.ts`; never hand-edit, never hand-write a DTO type.
- Regenerate with `pnpm gen:api` (the server prints its OpenAPI document with `memory-server openapi`; `pnpm gen:types` rebuilds only the types from `web/openapi.json`). `web/openapi.json` and the generated file are committed. The rust CI job fails when the server's document differs from `web/openapi.json`; the web job fails when `pnpm gen:types` changes the generated file.
- `openapi-typescript` (7.13.0, latest) drives the TypeScript 5 compiler API, which TypeScript 7 does not provide ([openapi-ts/openapi-typescript#2841](https://github.com/openapi-ts/openapi-typescript/issues/2841)). A pnpm override of its peer does not hold on a clean install, so the generator lives in its own workspace package, `web/tools/openapi-codegen`, with TypeScript 5.9.3 as its only compiler; the app keeps 7. Drop the package once a release supports 7.
- One `createClient<paths>()` instance in `shared/api`; entities wrap calls (`getPage`, `listVersions`).
- Wrappers return `data` and throw a typed `ApiError` when `error` is set; callers never inspect `response.status`.
- Request paths are static literal strings so types infer.
- Domain types are derived from generated ones: `type Page = components["schemas"]["Page"]`.

## Errors and loading

- Loading: route `pendingComponent` and `Suspense` boundaries with skeletons shaped like the final content; no spinners for content that is under ~200 ms.
- Errors: route `errorComponent` per page plus one in `app`; `QueryErrorResetBoundary` for retry.
- A failed mutation shows a toast with the server message; a failed read shows the error boundary.
- Empty state is its own component, separate from loading and error.
- Never swallow an error: no empty `catch`, no `catch` that only logs.
- Throw `Error` subclasses only; never throw strings.

## Styling

- Tailwind utilities only. No inline `style` objects (except dynamic values that must be computed, e.g. a CSS variable).
- Merge conditional classes with `cn()` from `shared/lib/cn` (`clsx` + `tailwind-merge`); no string concatenation.
- Colors, radii, spacing come from theme tokens in the Tailwind 4 `@theme` block; no raw hex values or arbitrary values (`w-[137px]`) in components.
- shadcn components are copied into `shared/ui` and edited there; app code never forks them inline.
- Variants use `cva`; no class logic in JSX conditionals beyond `cn`.
- Class order is handled by the formatter, never by hand.

## UI text

- Never use the middle dot or a bullet as a separator in user-facing text. Separate items with layout (spacing, separate elements, muted secondary text) or, where text must be inline, with commas, parentheses or plain words ("acme.pdf p.1", "claude-code, 2 hours ago").

## Accessibility

- Use semantic elements first (`button`, `a`, `nav`, `main`, `ul`); `div onClick` is banned.
- Every interactive element is keyboard reachable with a visible focus style.
- Icon-only buttons have `aria-label`; images have `alt` (empty when decorative).
- Inputs have an associated `label`.
- Use Base UI primitives for dialogs, menus, popovers, combobox; do not hand-roll focus traps.
- Never convey state by color alone.

## Testing

- Vitest for unit and component tests, colocated; Playwright for end-to-end flows in `e2e/`.
- Test behavior through the public API of a slice, not internals.
- Query by role and label (`getByRole`, `getByLabel`); `data-testid` only as a last resort.
- No snapshot tests for components.
- Mock at the network boundary, never mock Query or Router.
- Every bug fix lands with a failing test first.
- Test names are sentences: `it("restores the selected version")`.

## Comments

- Default is none. Allowed: one line explaining a WHY the code cannot express (an external constraint, a workaround with its reason).
- Never restate what the code does. No JSDoc blocks, no section dividers, no `// ---` banners, no commented-out code.
- Lint disables must carry a one-line reason after `--`.

Good:

```ts
const flushDelayMs = 0; // CodeMirror reads layout after the next frame; 0 defers past it
```

Bad:

```ts
// Get the page by id
function getPageById(id: string) { /* ... */ }

// ---------- helpers ----------
```

## Tooling

Decision and evidence (checked 2026-11-23):

- Linter: Oxlint with type-aware mode (`oxlint-tsgolint`). tsgolint went stable on 2026-07-22 with 59 of typescript-eslint's 61 type-aware rules and runs 12 to 18 times faster than ESLint + typescript-eslint (https://oxc.rs/blog/2026-07-22-type-aware-linting-stable). It runs on the real TypeScript checker, so `no-floating-promises` and `no-misused-promises` are accurate. Biome infers types itself, and its `noFloatingPromises` is still in `nursery` (https://biomejs.dev/linter/rules/no-floating-promises/).
- React rules: Oxlint ships `react/rules-of-hooks` and `react/exhaustive-deps` (https://oxc.rs/docs/guide/usage/linter/rules.html); both are off by default and enabled below.
- Formatter: Oxfmt. Passes 100% of Prettier's JS/TS conformance tests (beta announcement 2026-02-24, https://oxc.rs/blog/2026-02-24-oxfmt-beta), sorts imports and Tailwind classes built in, about 30x faster than Prettier. Risk: still 0.x (v0.79); the exact version is pinned in `package.json`. Fallback is Prettier with no config change in code.
- FSD boundaries: Oxlint `no-restricted-imports` overrides, no extra tool. Slices import their own files relatively, so any `@/<own layer>/` import is a sibling import and is banned; upward layers are banned per layer; deep imports past a slice's `index.ts` are banned everywhere except `shared`.
- Package manager: pnpm 11. Blocks dependency install scripts, enforces `minimumReleaseAge` of 1 day and blocks exotic subdependencies by default (https://pnpm.io/blog/releases/11.0), strict `node_modules`, Node-based like Vite and Playwright. Bun and npm have these protections opt-in only. Requires Node 22+.
- TypeScript 7 (native compiler, GA 2026-07-08, `latest` on npm is 7.0.2): `strict` is default, `baseUrl` and `moduleResolution: node` are removed (https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/). The command stays `tsc`. Do not copy the `baseUrl` line from the shadcn Vite guide.

Scripts:

```json
{
  "gen:openapi": "SQLX_OFFLINE=true cargo run --locked -q -p memory-server -- openapi > openapi.json.tmp && mv openapi.json.tmp openapi.json",
  "gen:types": "pnpm --dir tools/openapi-codegen exec openapi-typescript ../../openapi.json --empty-objects-unknown -o ../../src/shared/api/schema.d.ts",
  "gen:api": "pnpm gen:openapi && pnpm gen:types",
  "typecheck": "tsr generate && tsc --noEmit",
  "lint": "oxlint",
  "format": "oxfmt",
  "check": "tsr generate && oxfmt --check && oxlint && tsc --noEmit",
  "test": "vitest run",
  "e2e": "playwright test"
}
```

`tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "es2023",
    "lib": ["es2023", "dom", "dom.iterable"],
    "module": "esnext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "types": ["vite/client"],
    "rootDir": "./",
    "paths": { "@/*": ["./src/*"] },
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noImplicitOverride": true,
    "noFallthroughCasesInSwitch": true,
    "noPropertyAccessFromIndexSignature": true,
    "verbatimModuleSyntax": true,
    "isolatedModules": true,
    "erasableSyntaxOnly": true,
    "resolveJsonModule": true,
    "skipLibCheck": true,
    "noEmit": true
  },
  "include": ["src", "e2e", "vite.config.ts", "playwright.config.ts"]
}
```

- `noUncheckedIndexedAccess` is also required by openapi-fetch's docs.
- `erasableSyntaxOnly` bans enums and namespaces, matching the no-enum rule.

`.oxlintrc.json`:

```json
{
  "plugins": ["typescript", "react", "jsx-a11y", "import"],
  "categories": { "correctness": "error", "suspicious": "error" },
  "options": { "typeAware": true, "denyWarnings": true, "reportUnusedDisableDirectives": "error" },
  "rules": {
    "react/rules-of-hooks": "error",
    "react/exhaustive-deps": "error",
    "react/react-in-jsx-scope": "off",
    "typescript/no-floating-promises": "error",
    "typescript/no-misused-promises": "error",
    "typescript/no-explicit-any": "error",
    "typescript/consistent-type-imports": "error",
    "typescript/no-non-null-assertion": "error",
    "import/no-cycle": "error",
    "import/no-default-export": "error",
    "import/no-unassigned-import": ["error", { "allow": ["**/*.css"] }],
    "eqeqeq": "error",
    "no-console": "error",
    "no-warning-comments": "error"
  },
  "overrides": [
    {
      "files": ["*.config.ts", "src/app/routes/**"],
      "rules": { "import/no-default-export": "off" }
    }
  ],
  "ignorePatterns": ["dist", "src/shared/api/schema.d.ts", "src/app/routeTree.gen.ts"]
}
```

- `typeCheck` stays off; `tsc --noEmit` is the single source of type errors.
- Exact rule names such as `import/no-default-export` and `typescript/consistent-type-imports` follow Oxlint's ESLint-compatible naming; confirm each with `oxlint --rules` after install.

`.oxfmtrc.json`:

```json
{
  "printWidth": 100,
  "singleQuote": false,
  "trailingComma": "all",
  "sortImports": { "internalPattern": ["@/"], "newlinesBetween": true },
  "sortTailwindcss": { "stylesheet": "./src/app/styles/index.css", "functions": ["cn", "cva"] },
  "ignorePatterns": [
    "dist",
    "openapi.json",
    "src/shared/api/schema.d.ts",
    "src/app/routeTree.gen.ts"
  ]
}
```

FSD overrides in `.oxlintrc.json` (one per layer, generated from the same shape; verified with probe files):

```json
{
  "files": ["src/entities/**"],
  "rules": {
    "no-restricted-imports": ["error", { "patterns": [
      { "group": ["@/app/**", "@/pages/**", "@/widgets/**", "@/features/**", "@/entities/**"], "message": "FSD: entities imports only layers below" },
      { "group": ["@/shared/*/*/**"], "message": "FSD: import shared units through their index" }
    ] }]
  }
}
```

- The first pattern bans the layer itself and everything above, with `/**` so deep paths are caught too. A second pattern bans deep imports into lower slice layers (`@/entities/*/**` for `features`, and so on). The last bans deep imports into a shared unit (`@/shared/ui/button/x`) but allows `@/shared/ui/button`.
- `app` has no layer ban; it bans deep slice imports (`@/pages/*/**` and the other slice layers) and deep shared imports. `shared` bans only the layers above it.

- TanStack Router file routes live in `src/app/routes`, configured in `tsr.config.json` (read by both the Vite plugin and `tsr generate`); the generated `routeTree.gen.ts` sits in `app`, is gitignored, and is ignored by lint and format. `tsr generate` runs before `typecheck` and `check` because the tree does not exist in a fresh clone.
- shadcn: `npx shadcn@latest init -b base -p nova` (Base UI, preset `nova`); `components.json` aliases point `ui` to `@/shared/ui` and `utils` to `@/shared/lib/cn`. The CLI writes flat files, so move each added component to `shared/ui/<name>/<name>.tsx` with an `index.ts` and import `cn` from `@/shared/lib/cn`.
- CI runs `pnpm check`, `pnpm test`, `pnpm e2e` and `pnpm gen:types && git diff --exit-code`; the rust job compares `memory-server openapi` with `web/openapi.json`.
