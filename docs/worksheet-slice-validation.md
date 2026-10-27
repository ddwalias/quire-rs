# First worksheet slice validation

Date: 2026-10-27. This report covers the text/math editor, ordered real
calculation with length units, recalculation, and Quire v1 JSON persistence.
The implementation uses the existing `quire-core` and `quire` packages.
The core has no GUI, Windows, Mathcad, or licensing dependencies.

## Checks run locally

Environment: Windows 11 Enterprise x64, build 10.0.26300, Intel Core i7-1260P,
16 logical processors. `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, Cargo 1.99.1.

Passed:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
git diff --check
```

The suite has 22 tests: 6 desktop UI workflows, 2 core save-failure unit
tests, and 14 core integration tests. The workflow harness drives the exact
production egui UI with pointer/keyboard events and a real temporary
filesystem; it does not replace calculations or file services with mocks.
It runs without a window server/GPU, so it can also run on all CI platforms.

Coverage includes arithmetic precedence, scientific literals, ordered binding
and redefinition, upstream edits, failed definitions blocking stale values,
conversion and dimensions, unsupported syntax, division by zero, overflow,
bounded parser depth/size, Unicode text/path preservation, exact region order
and IDs, unsupported math source preservation, save/reopen/replacement,
unknown schema/content rejection, duplicate IDs/keys, malformed JSON/UTF-8,
read-only and missing destinations, simulated partial writes, and refusal to
open/save Mathcad paths. File failure checks compare original bytes.

The UI workflows also check moving/deleting regions, error recovery, save-as
replacement confirmation, failed open/save retaining current work, New with
unsaved changes, and cancellation of a window-close request.

## Native Windows workflow

`cargo run -p quire --locked` launched successfully. Through the native Windows
accessibility controls (Value and Invoke patterns), the running app:

1. Opened `corpus/lengths.quire.json` and displayed `125 cm`.
2. Changed the earlier definition to `a := 50 cm` and displayed `150 cm`.
3. Saved as `.cache/native-smoke.quire.json`.
4. Created a new worksheet, reopened the saved file, and displayed `150 cm`.
5. Closed normally with no unsaved changes.

The native window was visually inspected at the default 900 by 760 logical
size; all five fixture regions and their results were visible. The generated
worksheet and screenshot remain ignored local artifacts. This native smoke
check was performed during development; it is not yet an automated CI job.

## Reference evidence and fixture

See [the corpus record](../corpus/README.md) for the observed Prime 12.0.1.0
Express result, SI/automatic calculation settings, method and limitations.
The observation matches **1.25 m** for the two-definition example. All other
Mathcad behaviors, including errors, editing, formatting and file fidelity,
remain unverified. No Mathcad document is read or written by Quire.

Synthetic fixture: `corpus/lengths.quire.json`, SHA-256 (LF bytes):
`0f2dc8dab74d484bfc4b109a3dcd8e1c6a4caf78483654b346050dd60f5e2764`.
The `.gitattributes` rule keeps these line endings stable across platforms.
No original binaries, IL, raw reference captures or private worksheets were
added to source control.

## Initial core performance baseline

Command: `cargo run -p quire-core --release --example baseline --locked`.
Generated workload: 1000 math regions, starting with `x := 25 cm`, then 998
ordered increments of 1 mm, then conversion to centimetres. The program
checks the result, edits the first definition to 50 cm, recalculates, and
checks save/reopen equality. One local run on the environment above gave:

| Operation | Elapsed |
|---|---:|
| First calculation | 0.7133 ms |
| Edit and full recalculation | 0.7000 ms |
| Mean of 100 recalculations | 1.000727 ms |
| Save, including file sync | 4.1078 ms |
| Reopen (without calculation) | 0.8221 ms |

These are a starting observation, not performance guarantees or CI budgets.
Desktop startup, rendering performance, memory, and large worksheets have not
been benchmarked. Calculation currently rebuilds all bindings synchronously.

## Remaining platform and product gaps

- **Actually tested: Windows x64 only.** macOS and Linux builds, Clippy and
  the portable/egui workflow tests remain to be run by the existing three-OS
  CI matrix. Native desktop smoke workflows on macOS/Linux remain unrun;
  no runners for them were available in this Windows session.
- The declared Rust 1.95 minimum was not separately tested here; local checks
  used the toolchain recorded above, and existing CI uses stable.
- IME, screen-reader usability, clipboard behavior, font fallbacks, DPI
  variations and platform-specific rendering need further testing.
- No document-wide undo/redo, 2D math editor, functions, matrices, symbolic
  engine, printing, other dimensions or custom units. See README for syntax
  and numerical limits. No Mathcad import/export or preservation claim.
- Interrupted-write simulation covers a partial write and cleanup. Actual
  disk exhaustion, crashes during replacement and sudden power loss have
  not been induced. Atomic replacement depends on the host filesystem;
  directory-entry power-loss durability is not promised.
