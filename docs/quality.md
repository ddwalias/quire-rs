# Proposed product quality strategy for the Rust port

Status: draft for discussion. This strategy applies to the Rust implementation; the separate mirror project retains its own Release IL rules. Mathcad Prime 12.0.1.0 is the initial reference. Windows, macOS and Linux are first-class targets. Reference and cross-platform workflow tooling described below is planned, not implemented yet.

The [QA operating plan](qa-plan.md) defines feature verification, defect triage,
evidence records and release qualification from the first milestone onward.
It distinguishes existing CI configuration from enforcement and automation
that still need implementation.
The [backlog review](backlog-review.md) records the finite-scope readiness
requirements and separates current-product QA from future-feature dependencies.

## 1. Quality objective

Users must be able to trust both their engineering results and their files. A feature is ready to claim when its supported inputs and limits are clear, its results and errors have suitable evidence, it behaves correctly on supported platforms, and editing or saving does not silently destroy worksheet content. UI fidelity, numerical compatibility, document fidelity, reliability, accessibility and performance are separate dimensions; do not collapse them into one percentage.

Adopt the useful parts of [PhotoCraft's published workflow](https://github.com/storytold/photocraft/blob/main/AGENTS.md): real-file corpora, reference comparisons, application-level automation, crash prevention and checks proportionate to the area being changed. This is an inspiration, not evidence that PhotoCraft has reached full parity. Quire adds engineering-specific checks for units, calculation order, error behavior, precision and significant figures.

## 2. The quality loop

1. Choose a user-visible feature and representative reference worksheets; state supported inputs and important edge cases.
2. Record observed Mathcad behavior or another defensible oracle, including source/version/settings and known uncertainty.
3. Implement the feature in a cross-platform vertical slice, including failure and unsupported cases.
4. Run focused automated checks, the relevant end-to-end cases, and Windows/macOS/Linux checks. Compare semantic outputs to the reference where available.
5. Preserve passing cases as regression tests. Fix unexpected differences; document intentional limitations. Do not mark unknown expected outcomes as passing.

Use judgment about test scope: a numerical algorithm needs stronger numerical and edge-case evidence than a text label. Tests must check meaningful behavior, not just repeat implementation details. Exact IL parity remains a separate concern for the existing mirror project; the Rust port cannot meet it.

## 3. The worksheet corpus

Maintain a small, versioned suite of synthetic `.mcdx`/Quire worksheets, hand-checked examples and real-world files where their use is permitted. Group cases by feature: arithmetic; repeated definitions and evaluation order; unit conversions and dimensional errors; arrays/matrices; functions; symbolic operations; plotting; and document editing/preservation. A manifest records case ID, file hash, origin, allowed handling, Prime version, locale, unit system, expected behavior, comparison rules and known unsupported content. Restricted original files and outputs stay in the ignored local cache; CI uses synthetic or explicitly permitted fixtures.

For real files, test opening, navigation, editing, calculation and saving, not merely successful parsing. A no-op open/save should preserve unsupported parts, relationships and embedded data where that has been verified. Compare package contents semantically; byte-identical ZIP output is not required unless a feature explicitly requires it. If preservation cannot be assured, keep the file read-only or warn/refuse the unsafe operation instead of claiming losslessness. Never replace the original after a failed write.

Start with a dozen or so **representative**, small cases; grow based on observed failures and user worksheets, not a target number. Each real regression becomes a minimal reproducible fixture where possible.

## 4. Mathematical correctness and reference comparisons

Run supported cases against the installed Mathcad reference on Windows where automation or another reproducible method is available. Capture the exact input, output value/shape, units and dimensions, formatting settings, errors, Prime version and environmental dependencies. On other platforms replay permissible fixtures and expected semantic results. Where the oracle cannot be automated or observed, label the case unverified; use independently calculated examples, analytic identities or additional reference implementations as supporting evidence, without presenting them as Mathcad parity.

Comparison depends on the contract:

- Exact comparisons for discrete values, result type/shape, units/dimensions, execution order, errors and stable document structure where appropriate.
- Function-specific absolute, relative or ULP tolerances for finite floating-point results, justified by the feature and reference behavior. Record the tolerance per test; do not use one blanket epsilon.
- Explicit cases for NaN/infinity, signed zero, overflow, domain boundaries, singular/ill-conditioned matrices, complex branches, ranges, vectorization and randomized functions. Seed and specify generators when reproducibility is required.
- Separate tests for expression evaluation and display rounding/significant figures; a pretty string must not hide a wrong underlying value.
- Unit cases include incompatible dimensions, custom units, fractional exponents, affine temperatures, conversions and matrices containing quantities.

Example first case: define a length in centimetres, add a length in metres, evaluate in metres, and compare the numeric value and dimension separately. Also edit the upstream definition and verify that dependent results update in the observed worksheet order.

## 5. UI and complete workflows

Create a small automation interface to open a worksheet, insert/edit/move a region, trigger calculation, inspect a semantic result, save and capture a screenshot. It should exercise the same application services as a real user; it is a testability interface, not a substitute for external COM compatibility. Test keyboard navigation, mouse selection, undo/redo, clipboard, IME, accessibility and error recovery on supported platforms.

Use semantic assertions for region identity, expression, selection, result and error first. Use screenshots to detect layout problems and meaningful visual regressions, with platform-specific baselines, pinned fonts, theme and DPI. Screenshot similarity is not evidence that a calculation is correct. Periodically inspect the application manually for workflows automated tests cannot judge well.

## 6. Reliability and cross-platform behavior

The first vertical slice must build and run on Windows, macOS and Linux. CI runs formatting/lints and portable unit/integration tests on all three; the reference-oracle job runs on Windows when a licensed installation and test environment are available. GUI smoke tests run on each supported platform when their runner can support them; clearly report skips. Windows-only COM/OLE/native-DLL adapters are optional and tested separately; unsupported objects are preserved and visibly identified on other platforms.

Exercise malformed XML/ZIP, oversized entries, path traversal, missing resources, cancellation, interrupted writes, read-only locations, full disks and crashed external integrations. Use generated malformed inputs and fuzzing where parsers/FFI warrant it. A fault should produce a useful error; if a crash occurs, recover work where possible and add a regression. Save through a temporary file and replace the destination only after successful completion.

Compare behavior across OSes and architectures that the project actually supports. Pin or record locale, time zone, fonts, CPU/solver settings and dependency versions for tests where they affect results. Detect stale asynchronous results with document revision IDs; test edits made during a calculation.

## 7. Performance and release checks

Measure, on named machines and representative worksheets: app startup, file open/save, first calculation, recalculation after a small edit, memory use and rendering/interaction responsiveness. Capture baselines before setting budgets. Run quick representative checks for performance-sensitive changes and more stable benchmarks periodically or before release. Investigate regressions against the measured baseline; do not promise a numeric threshold until the corpus and hardware are defined.

Before a release, verify the agreed supported-platform matrix, the core worksheet corpus, cross-platform build/install, save/reopen and recovery, accessibility smoke checks, compatibility notes, and resolved critical defects. List known unsupported features and visible differences. Check third-party dependency provenance and packaging. Do not claim complete Mathcad parity based on a test count, source coverage or a UI checklist.

## 8. Suggested automation and ownership

Keep commands simple and add them only when the Rust workspace exists:

- `cargo fmt --check`, `cargo clippy` and `cargo test` for the relevant workspace/packages; CI runs the agreed full suite.
- A `quire-compat` test runner for corpus manifests, reference observations and comparisons. Separate a Windows oracle capture from portable replay.
- A headless CLI or app control interface for workflow tests and screenshots.
- Targeted parser fuzzing and performance benchmarks for risky components; neither is a substitute for end-to-end cases.

Treat the corpus, reference capture and comparison code as production quality infrastructure: review expected-value updates, identify the reason for each changed expectation, and never silently overwrite a reference because a test failed. A current limitation or external service outage is recorded as such, rather than being counted as an application pass.

## 9. First implementation milestone

Deliver the first cross-platform vertical slice with at least:

1. A synthetic worksheet with text, two definitions, a unit conversion and an evaluated result.
2. A reference observation from Mathcad with the version and unit settings recorded.
3. Automated checks for calculation order, result/dimension, editing and recalculation.
4. Save/reopen tests for Quire's format, and read-only import/preservation checks for the selected Prime file.
5. One end-to-end UI flow on each supported operating system, with explicit skips where a runner cannot execute it.
6. Failure tests for invalid input and interrupted save, and a basic performance measurement to establish the first baseline.

This is a starting bar for that slice, not a blanket claim that other worksheet features are supported.
