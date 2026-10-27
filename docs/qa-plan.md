# Product QA operating plan

Established 2026-10-27. This plan applies from the first milestone onward and
implements the workflow described in [the quality strategy](quality.md).
The process and checklist are documented; automation and repository enforcement
remain tracked work until verified. This document is not a test execution report.

## Ownership and cadence

Each feature has an implementation owner and a named verifier before entering
In Review. The verifier checks evidence and exercises the relevant workflow;
numerical, persistence and release-critical changes should receive a second
person's review when available. If only one contributor is available, record
self-verification explicitly and keep independent release review outstanding.
The project manager tracks missing evidence and blockers. A release owner
records the release decision; role names do not imply staffing is assigned.

Review quality readiness at each milestone planning/review and before every
release candidate. Triage new defects during active development; treat silent
wrong calculations, data loss and reproducible crashes as immediate priorities.

## Feature workflow and definition of Done

1. **To Do:** define supported behavior, excluded inputs, risk and acceptance
   criteria. Identify representative fixtures and an expected-result source.
2. **In Progress:** implement a complete workflow with appropriate unit,
   integration or application tests. Record reference observations separately
   from independently calculated expectations.
3. **In Review:** link the commit/build, changed behavior, test evidence and
   named verifier. Review edge cases and failure paths, not just the happy path.
4. **Done:** acceptance criteria pass for the declared scope, relevant checks
   pass, regressions are covered, limitations are documented and the verifier's
   result is recorded. An unrun required check leaves verification incomplete.

A limited feature can be completed with an explicit supported scope. Missing
Mathcad observations must remain labelled unverified; they cannot establish
compatibility. Optional or unsupported behavior is never counted as passing.
Restricted worksheets, reference outputs and binary research stay local.

Attach this evidence record to the feature ticket or its linked PR:

```text
Feature / ticket:
Commit or candidate build:
Supported scope and acceptance criteria:
Risk areas (calculation, files, interaction, platform, performance):
Case IDs / fixture provenance:
Expected-result source, reference version and settings:
Comparison rules and tolerances:
Commands / workflow steps and outcomes:
OS, architecture, toolchain and relevant locale / DPI / fonts:
Passed / failed / blocked / not run / out of scope:
Defects, unsupported behavior and remaining risks:
Verifier and date:
```

## Test selection

| Risk | Minimum relevant evidence |
| --- | --- |
| Calculations and units | Values, types/shapes, dimensions, ordering, domain/error boundaries and suitable numerical tolerances. Use observed reference cases when claiming compatibility; analytic identities or independent checks provide additional evidence. |
| Document handling | Save/reopen, migration, malformed input, preservation of unsupported content and failed-write protection of original bytes. Keep Prime files read-only until preservation is proven. |
| Editor and canvas | Semantic application workflows for editing, recalculation, navigation, selection and undo where supported; targeted visual review for layout changes. |
| Desktop integration | Native launch/open/edit/save/reopen/close on Windows, macOS and Linux; include keyboard, IME, accessibility and DPI checks where affected. Headless egui tests do not establish native platform behavior. |
| Parsers and external integrations | Resource bounds, malformed input, cancellation, failure isolation and targeted fuzzing where warranted. |
| Performance | Repeatable measurements on named hardware and representative worksheets. Establish budgets from measured baselines. |

Select tests by impact. Do not require a coverage percentage or add tests that
only repeat implementation details. Property or metamorphic checks are useful
for meaningful invariants such as unit conversion round-trips, solver residuals
or incremental/full recalculation equivalence; they do not prove Prime parity.

## Automated checks and their status

The existing `.github/workflows/ci.yml` configures these checks on Windows,
macOS and Linux:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Record actual run links and results for the relevant commit. A workflow file
does not prove a successful run or required branch protection. Required PR
checks, Rust 1.95 validation, portable compatibility replay, native desktop
automation and durable QA reporting are tracked implementation work. While
automation is unavailable, record reproducible manual checks and their limits.
Native manual checks may satisfy an individual workflow check; they do not
mark the automation deliverable complete.

## Exploratory QA

For each affected workflow, write a short charter naming the build, platform,
feature risk and intended tasks. Exercise realistic sequences: edit a definition
and its dependents, move regions across calculation order, save/reopen, cancel
dialogs, handle invalid input and recover from failures. For file changes,
retain the original and compare its contents after failed operations.

Log observations, unexpected behavior and reproducible steps against the build.
Include keyboard-only and relevant accessibility/IME/DPI paths. Link defects to
the charter. Repeat failed cases on the fixed build and add a focused automated
regression when practical. Mark unavailable platforms blocked or not run.

## Defect and flaky-test policy

| Severity | Example and disposition |
| --- | --- |
| Critical | Silent wrong results, corrupted/lost worksheets, or a severe exposure. Stop the affected release path and investigate immediately; block release until fixed and verified. |
| High | Supported core workflows crash or are unusable without a practical workaround. Block release until fixed and verified. |
| Medium | Incorrect behavior with a practical workaround or a limited feature impact. Fix or record a release-owner decision with affected scope and limitations. |
| Low | Minor presentation/usability issue with no material result or file impact. Prioritize normally and disclose when relevant. |

Severity describes impact; ticket priority describes scheduling. Each defect
records build/platform, expected versus actual behavior, reproduction steps,
permitted fixture, severity, affected feature and regression verification.

Do not turn a red run green by hiding intermittent failures. Preserve the
failing run and reproduction seed/environment. Quarantine requires a linked
defect, owner, reason, review date and an explicit coverage gap. A rerun does
not erase the failure, and a quarantined required case cannot satisfy a gate.
Review reference/visual baseline updates with their reason; never overwrite
expectations simply to make a test pass.

## Release qualification

Qualify an immutable candidate commit and artifact checksums. Record the
supported platform/feature matrix, CI runs, corpus results, native workflows,
installation, save/reopen/recovery, accessibility checks and performance
comparisons applicable to that release. Store only permitted evidence.

Every required case must be passed. Failures, blocked cases and unrun cases
remain visible and prevent qualification for that scope. Remove unsupported
scope explicitly and update claims rather than mark its tests passed. Critical
and High defects block release. Record decisions on lower-severity defects.

The release report names the verifier and release owner, links outstanding
defects and states pass/fail/blocked by platform and feature. Rerun checks
affected by a candidate change; tie all evidence to the final candidate.
Publish supported behavior and known limitations without a single parity score.

## Tracking

Kaneo's product quality epic, **QUI-11**, owns this operating process as well
as the existing accessibility, native smoke, robustness and release work.
Existing evidence infrastructure remains in QUI-15 (corpus), QUI-16 (reference
capture) and QUI-17 (portable comparison). QUI-93 owns native smoke automation,
QUI-95 fuzzing, QUI-96 performance and QUI-98 minimum-Rust validation.
QUI-100 prepares the release scope manifest and compatibility notes before
QUI-121 records the candidate qualification decision. QUI-122 owns integrated
engineering/user acceptance and QUI-123 owns release getting-started guidance.

The operational deliverables are QUI-116 (feature verification and Done),
QUI-117 (required CI checks and run evidence), QUI-118 (feature/test evidence
reports), QUI-119 (exploratory QA), QUI-120 (defect and flaky-test triage), and
QUI-121 (release candidate qualification). Their execution remains open.
QUI-121 consumes QUI-100, the candidate artifacts and integrated acceptance;
documentation does not depend on a qualification decision that needs it as input.

The [backlog review](backlog-review.md) records the product and dependency
corrections. M1/M2/M3 are provisional sequencing labels; QUI-13 must record the
finite release contract. Current-platform QA can begin on the existing product
while new editors, importers and scientific capabilities are developed.
