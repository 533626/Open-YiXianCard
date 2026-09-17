<!-- topic: maintenance -->

# Simplification review and remaining work

## Status

The target was revised by the user from 30% to **15%** overall source reduction.
This remains a partial delivery, not completion of that target.
The interrupted parallel changes have been repaired, reviewed in batches and committed locally.
No remote publication is required by this tracking note.

## Fixed measurement scope

Baseline: `c15baaa64a4c87c9d2872363ea7e74d037e1f224`.
Measured implementation: `c64272f1c06b8f2726c9184eed7cfeeda8000ce6`.

Count Git-tracked physical source lines (including comments/blanks) for `.rs`, `.ts`, `.tsx`,
`.js`, `.jsx`, `.mjs`, `.cjs`, `.py`, `.css`. Include tests, embedded code data, new modules and
frozen `engine-ts`. Exclude dependency directories (`node_modules`, `target`, `venv`, `.venv`,
`__pycache__`, `.git`) and `public/build/`, `out/`, `logs/`, `.rotation/` prefixes.
Do not use working-directory traversal: dependency installations and baseline worktrees distort it.

| Surface | Before | After |
| --- | ---: | ---: |
| Rust | 74,650 | 73,273 |
| Frozen TS | 71,709 | 71,709 |
| Analysis | 66,205 | 66,023 |
| Evaluator | 69,082 | 67,329 |
| UI/source | 30,666 | 30,510 |
| Scripts | 10,285 | 10,263 |
| Research | 15,110 | 15,110 |
| Practice skill code | 5,251 | 5,251 |
| Shared + root JS | 134 | 134 |
| **Total** | **343,092** | **339,602** |

Net removal: **3,490 lines (1.02%)**. To achieve at least 15%, total must reach **291,628**
or fewer: **47,974 further lines** would have to be removed from the measured implementation.
The continuation after the initial review removed **1,671 additional lines**.
Splitting a file does not count as removing its extracted code.

## Local commits and verified repairs

- `a4c6a42d05`: isolate turn-end observation helpers; restrict visibility to replay siblings;
  update the static handler index's moved source line without changing routes or counts.
- `38ab65b3d6`: share fixture/card builders and resource counter writers. Review caught the
  difference between padding and `Vec::resize_with` truncation; separate `fill_deck` and
  `resize_deck` preserve the old callers' behavior. Added oversized-deck regression coverage.
- `08a2ab1444`: shared TUI smoke assertions preserve interpolated diagnostics and lazy evaluation.
- `9a31eb61c2`: split popup, battle-render and verdict responsibilities; preserve replay CLI
  conflict wording and restore two assertions omitted by the original test consolidation.
- `76f5b9e656`: UI dispatch table and unused presentation helper removal. Unknown action keys
  must not dispatch inherited Object methods. The targeted regression fails without the own-key
  guard and passes with it. Browser smoke and self-audit passed.
- `83d2240ab0`: evaluator canonical serialization, hash and admission helper consolidation.
  Preserve ordinal versus locale sorting at each caller; do not invalidate evidence hashes.
- `d4ac427db7`: Analysis statistics, CLI dialect parsing, reachability helpers and lambda backends.
  The canonical lambda entry points accept `--evaluator ts` instead of separate legacy entry points.
  Review restored empty-token and equals-token behavior separately for all three parser interfaces.
- `6e774a4b10`: share strict event checkpoint field validation without relaxing parity.

TUI entry-file size improvements: `popups.rs` 1,424 → 213 lines and `render_battle.rs`
1,263 → 71. Popup module total is 1,467 lines, so the popup split improves navigation,
not total LOC. Verdict computation, rendering and shared types now have separate modules.

## Continuation under the 15% target

- Shared private/public replay-focus and original-build-profile test suites retain their
  registrations. Profile consolidation also corrects private-suite drift for build `25206201`.
- Historical regression manifests share a builder. Two unconditional SHA-256 tests preserve
  the full serialized manifests for builds `24180265` and `24217566`, each with 706 cases,
  353 pairs and 277 targets.
- Shared card-batch scaffolding now serves batches **005, 006, 007, 011, 012, 013, 014**.
  Each conversion was checked against its pre-change complete JSON serialization.
  Batch-003 was attempted and restored; it is **not converted**. Batch-010 is talent-shaped
  and was not forced into card scaffolding. Failed temporary converters are not project tests.
- The seven GA text reports now share one ordered deck label implementation
  (`orderedDeckReportLabel` in `ga-evolution-report-labels.ts`). Commit `d9b8593e2c`:
  +37/-69 (**-32 net lines**). Analysis type-check diagnostics stayed at 46 before and
  after (verified via stash A/B), and the GA suite shows **no new failures** versus its
  baseline run (9 pre-existing failures in both runs, unchanged names).
- Identified but **not** executed as source-LOC reductions: two other byte-identical
  helper groups across the same GA reports (parse-backend/tier family and the
  budget/uniqueBaseCards cluster, roughly 60 more lines), and 54,071 lines of
  byte-identical JSON mirrored between `battle-evaluator/` and `shared/data/`.
  JSON is outside the fixed source-LOC scope, so consolidating it must be justified
  by storage/provenance benefits alone.
- The final batch-005 commit `c64272f1c0` alone removes **94 net lines** (+22/-116).
  Its 26 cases were rechecked byte-for-byte against its parent commit after all shared-helper
  changes. This small result is not evidence that another batch sweep can achieve 15%.
- Final evaluator type check and `bun run check` pass. Historical fingerprint tests:
  **2 passed, 0 failed, 14 assertions**. Evaluator private/public run:
  **483 passed, 15 skipped, 67 failed, 4 errors**; failure names match the same-scope
  pre-013/014 baseline. This is **no new failures**, not an all-green test suite.
- Existing evaluator blockers include current-build fixture/evidence availability and
  `unclassified full-scope FateStrategy: 354`. The frozen TS classification is not edited.

Measurement evidence: `logs/practice/simplification-recovery/global/source-loc-15pct.json`.
Final checks: `final-15pct-check.log`, `batch005-final-ab.log`, `evaluator-after-batch005.log`
in the same ignored recovery directory. No original-client battles were executed by the
runtime manifest-generation probes; they only produced serialized runtime input files.

### Larger-module removal screening

The largest inspected files are not proven dead code:

- `full-match-loop.py` is dynamically loaded by `analysis/value/offline_driver/driver.py`
  and practice characterization tests; it invokes `drive_original_client.py`.
- `drive_original_client.py` remains the documented practice entry point and is exercised
  by `research/original-game/test_restart.py`.
- The campaign-evidence implementation remains behind package report/check commands.
- Rust card, flow, resource and player modules are registered in `engine-rust/src/replay.rs`.

These consumer checks rule out simply deleting those modules; they are not an exhaustive
reachability audit. Frozen `engine-ts/src/cards/*` remains untouched and counted. Stop the
low-yield batch-conversion sweep; further work needs larger, evidence-backed equivalence or
unreachability findings rather than wholesale removal of live functionality.

## Blocked: current-build solver fixture inputs

**Scope:** `analysis/tests/solver.test.ts`, current Steam build `25268934`.
**Status:** blocked; do not weaken assertions or substitute withdrawn/older-build evidence.

After rebuilding `solve_deck` and `solve_deck_batch`, the suite reports **4 passed / 16 failed**.
The remaining failures stop in canonical fixture selection:
`canonical replay selection found 0/1 fixtures for Steam build 25268934` (larger requested
fixture counts can appear in other suites). Files existing locally are insufficient: inputs
must satisfy current-build admission, withdrawal and selection contracts.

Before rebuilding, there were 17 failures. The extra tape-exhaustion fallback test recovered
when stale binaries were rebuilt; it is **not** part of the missing-fixture blocker.
Running the solver suite with the Analysis/scripts edits temporarily stashed reproduced the
17 failures at HEAD in the same working environment. The stash was restored successfully.
A separate clean baseline without release binaries skips some tests and is not an equivalent
pass/fail comparison; do not mistake skipped tests for passing ones.

Resolution requires correctly admitted, current-build fixture inputs with provenance preserved.
Then rerun `bun test analysis/tests/solver.test.ts` and require its exact assertions to pass.
No fixture admission, report build-label replacement or assertion relaxation was performed in
this simplification phase.

## Other gate findings

- `bun run check` passed after regenerating the one moved handler-index source line.
- Rust full tests: 728 library, 9 replay CLI, 37 TUI and 1 integration test passed.
  Clippy covering lib/tests with `-D warnings` passed. TUI smoke reported `replayExact=true`.
  This smoke is not a claim of complete private corpus admission.
- UI: 255 tests, type checking, build, browser smoke and 1600×1000 self-audit passed.
- Evaluator: type checking and 71 focused helper/admission/campaign tests passed.
- Analysis/reachability/talent/combo/ledger plus event-parity focused run: 83 passed.
- Analysis type checking still has **46 diagnostics**, byte-identical to clean baseline.
- Broad cross-surface run was **1,202 passed / 108 failed / 6 errors / 15 skipped** before
  subsequent targeted fixes and rebuilds. It has not been certified green.
- Stored combat semantic report uses build `25093011`; current generation uses `25268934`.
  The old and extracted SHA-256 implementations produce identical current-registry hashes.
  Do not change just the report's build string/hash to conceal stale evidence metadata.

Detailed local logs and recovery snapshots: `logs/practice/simplification-recovery/` (ignored).

## Remaining project review and simplification targets

No claim of exhaustive per-function review is made. Broad tests and structural inventory are
not substitutes for a full behavior review of untouched modules.

1. Practice orchestration: `full-match-loop.py` (3,572 lines) and
   `research/original-game/drive_original_client.py` (1,645). Separate phase execution,
   resource policy and state decoding only after driver characterization tests; retain timing
   and fail-closed mechanism boundaries.
2. Rust replay: `cards_dream_mirage.rs` (1,582), `flow.rs` (1,540), `replay.rs` (1,520),
   `resources.rs` (1,504), `cards_mirage_ronghui.rs` (1,472), `player.rs` (1,412).
   Shared declarative mechanics must preserve effect ordering and every exact outcome.
3. CLI and evaluation: `replay_slice.rs` (1,370), campaign evidence (1,285),
   `engine-event-parity.ts` (1,066). Separate wire schemas, command parsing and orchestration;
   retain strict nullability and omitted-field checks.
4. Offline driver (1,137 lines) and Analysis report programs still need broader consolidation.
5. Added generic helpers should have distinct responsibilities rather than growing another
   miscellaneous god module. Remove obsolete wrappers only after all consumers are traced.

The revised 15% request remains open. Do not meet it by deleting coverage, generated evidence, public
interfaces, the frozen TS archive, or by compressing formatting. Any future genuine removal must
be backed by reachability/consumer evidence and the affected contract tests.
