<!-- topic: product-architecture -->

# Public boundary

The development repository is intentionally private and keeps the complete engineering surface on one `main` branch. Public source is a separately generated allowlist projection; the private development tree must never be pushed or mirrored as the public repository. This change stops before release or deployment.

## Public source surface

The public surface is the canonical Rust battle engine and its WebAssembly/native build, the TypeScript compatibility archive (source/scripts type-check through `engine-ts/tsconfig.public.json`), neutral contracts and reviewed data (public type-check through `battle-evaluator/tsconfig.public.json`), stable rule-development documentation, and the browser UI. The development `main` may contain analysis, replay corpus, private tests, and terminal TUI, but the public projection must not require or contain them.

The browser is local-only and fail-closed. It starts from a blank build or an explicitly selected local input. Production static artifacts contain zero repository fixtures and no fixture index; `scripts/public-bundle-boundary.ts` and the release audit enforce that boundary.

## Private development surface

The following remain private engineering material on `main`:

- `analysis/` and analysis-only reports/data;
- private replay inputs, admission receipts, client-oracle material, mirror corpora, and derived evidence;
- `engine-rust/src/bin/tui.rs`, `engine-rust/src/bin/tui_app/`, and `engine-rust/tui-builds/`;
- the original-game research Python toolchain under `research/original-game/*.py` (client inventory/decode/decompile/index orchestration) and the build authority inputs `battle-evaluator/data/current-build.ts` / `original-build-profiles.json` — the reviewed data products they produce stay public, the toolchain itself is companion-only;
- operational handoffs and analysis-specific reports removed from the public documentation index.

The checked-in [`public-export-policy.json`](../public-export-policy.json) is the authoritative public projection allowlist. `bun run export:public -- --target /tmp/public-export` creates the export from a clean commit and validates that private paths are absent. `PRIVATE_ENGINEERING_EXTRACTION.json` remains only as a compatibility manifest for older companion backups; daily development does not require a second source directory.

Both public deliverables are fail-closed at 100,000,000 bytes: `audit.maxExportBytes` limits the complete source projection (including `public-export-manifest.json`), while `auditReleaseArtifact` applies the same ceiling to the complete static artifact (including `release-manifest.json`). Dependency caches and `.git` metadata are not public deliverables and are outside these byte counts.

## Corpus policy

Replay-derived fixtures are not covered by the project MIT distribution. They must be retained in a private evidence store, not silently deleted, copied into public history, or bundled into `dist/`. Rust corpus-dependent tests are explicitly gated behind the non-default `private-fixtures` feature and are maintained only in the private companion. See [`CORPUS_POLICY.md`](../CORPUS_POLICY.md) and [`NOTICE`](../NOTICE).

## Release and deployment boundary

`bun run build:site` and `bun run check:release` are local artifact operations. They do not publish a GitHub release or deploy Cloudflare. The existing workflow must remain disabled or separately owner-approved until the corpus history migration and public policy review are complete. No public URL is claimed.

## Next action before public push

Create a fresh public source export with `bun run export:public`, run the public-boundary and release checks in the exported clean tree, and obtain an explicit review of licensing, provenance, and generated references. Only after that review may a human decide whether to push a public repository. Do not force-push or delete the existing remote branch as part of this staging.
