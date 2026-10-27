# quire-rs

Quire is a cross-platform engineering worksheet application in early development.
The goal is a Rust application that runs on Windows, macOS and Linux and can
work with Mathcad Prime worksheets through independently implemented behavior.

Quire has a paged worksheet canvas with positioned text and mathematical
regions, a grouped ribbon, a worksheet grid and zoom. Equations display
typeset fractions, powers, italic variables and inline results. Ordered
definitions, real arithmetic, length units and automatic recalculation work
underneath the editor.

**Math editing still uses a source field in the selected region.** This is
not yet a structural equation editor with Mathcad-style cursor navigation.
Worksheets save in **Quire's `.quire.json` format, version 2**, including page
positions and text widths. Existing version 1 files open and migrate without
changing their source, IDs or calculation order.

**Mathcad files cannot be imported, edited, or saved.** Their structure and
preservation have not been validated. One small calculation was observed in
Prime 12.0.1.0 Express; this does not establish general Mathcad compatibility.

## Run

Install the stable [Rust toolchain](https://rustup.rs/), then run:

```sh
cargo run -p quire
```

On Linux, the desktop build also needs development libraries for the selected
windowing backends, including X11, Wayland and xkbcommon. The CI workflow lists
the packages installed on Ubuntu.

## Try a worksheet

Click **Example worksheet** to explore a positioned document with text, unit
conversions, fractions and powers. To start from scratch, click the page to
place a crosshair, then type a formula or choose **Math** or **Text** on the
ribbon. Use a separate math region for each line:

```text
a := 25 cm
b := a + 1 m
b -> m
b -> cm
```

The last two results are `1.25 m` and `125 cm`. Edit the first definition to
`a := 50 cm`: they become `1.5 m` and `150 cm`. Definitions apply only to later
regions; a redefinition affects subsequent uses. An invalid definition blocks
subsequent uses of that name instead of reusing its old value.

Click a region to select it; double-click it or choose **Edit** to edit its
source. **Esc** or **Ctrl/Cmd+Enter** returns to the typeset display, retaining
the current edit. Drag the selected region's left handle to move it. Resize
text using its bottom-right handle. Arrow keys nudge a selected region by
eight document units; Shift+arrow moves one unit. **Move up** exchanges its
position with the preceding region; **Delete** removes it. Calculation follows
page order, top to bottom and then left to right; moving a definition below
its use therefore produces an undefined-variable error.

The **Math** ribbon inserts operators at the formula cursor. **Document**
controls the grid and actual-size zoom; **Calculation** recalculates the
worksheet explicitly. Recalculation also happens automatically as you edit.
The status bar zooms from 65% to 150% without modifying saved coordinates.
Ctrl/Cmd+M and Ctrl/Cmd+T insert math/text; Ctrl/Cmd+S saves, Ctrl/Cmd+O opens,
and Ctrl/Cmd+N starts a new worksheet.

**Open…** and **Save as…** show a file-path dialog. Enter a path ending in
`.quire.json`, then submit it. Paths can be absolute or relative to the
directory from which Quire was started. These are currently path-entry
dialogs, not native file browsers. **Save** writes the current file, or asks
for a path when the worksheet has no file yet. The included examples are
`corpus/canvas-example.quire.json` (v2) and `corpus/lengths.quire.json` (v1).
Saving to a different existing destination requires explicit replacement.
New, Open, and closing the window ask before discarding unsaved changes.
Choose **Keep editing** to cancel that action and save first.

## Supported syntax and limits

- Real decimal/scientific literals, parentheses, unary `+`/`-`, and `+ - * / ^`.
  Powers associate rightward and bind before unary minus: `-2^2` is `-4`.
  Exponents must evaluate to dimensionless integers from -16 to 16; `0^0`
  and zero to a negative power are errors.
- Case-sensitive ASCII identifiers, such as `length_1`, with `:=` for
  definitions. Use explicit `*` between variables and around compound unit
  expressions; `25 cm` is shorthand for `25 * cm`. Multiplication, division,
  and unit shorthand have equal precedence and associate leftward: write
  `2 m / (4 cm)` when the entire length is the denominator.
- Reserved length units: `m`, `cm`, `mm`, `km`. Quantities use metres internally;
  products and quotients track integer length dimensions from -64 to 64.
  `-> m`, `-> cm`, `-> mm`, and `-> km` convert length results. Addition and
  subtraction require matching dimensions. Area and inverse-length results
  display as powers of metres; conversion of those dimensions is not supported.
- Unknown syntax/names, invalid dimensions, division by zero and non-finite
  results produce visible errors. Finite values use binary `f64` arithmetic;
  display uses Rust's shortest round-tripping decimal representation, without
  configurable precision. Very small values can underflow to zero.
- Maximum saved file size: 8 MiB. Math evaluation is limited to 16 KiB per
  region, 2048 tokens and 128 parser/tree levels. Invalid/unsupported math
  source remains editable and is preserved by save/reopen.

The JSON envelope identifies `quire-worksheet`, version `2`. It stores title,
ordered text/math source, stable IDs, page numbers, positions and widths;
results are recalculated on open. Page dimensions use fixed A4-like document
coordinates independent of screen DPI and zoom. There is a limit of 1000
pages. Version 1 files receive deterministic positions in their original
order and save as version 2. Earlier Quire builds cannot open version 2.
Unknown fields, versions or region kinds, malformed JSON, invalid layouts,
contradictory visual/calculation ordering and duplicate IDs are rejected so a
later save cannot silently lose unrecognized content or reorder calculations.
Save serializes and validates first, writes and syncs a temporary file beside
the destination, then replaces it. Failed writes leave the original intact.
Symbolic-link destinations and read-only files are refused. Directory-entry
durability across sudden power loss is not guaranteed on every filesystem.

Long previews are clipped with a visible indication; their full source stays
editable and is preserved. Text source editors scroll vertically.
There is no structural math editing, functions, arrays, symbolic calculation,
other base dimensions, custom units, printing, document-wide undo/redo,
multi-document tabs, configurable paper size or collision avoidance.
Recalculation is synchronous and rebuilds the whole worksheet in order.

## Develop

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p quire-core --release --example baseline --locked
```

`quire-core` holds UI-independent worksheet data; `quire` is the desktop shell.
We will split packages when implementation shows a useful boundary, not to
match Mathcad's DLL count. See [architecture](docs/architecture.md),
[quality strategy](docs/quality.md), [slice validation](docs/worksheet-slice-validation.md),
[canvas validation and UI research](docs/worksheet-ui-validation.md),
[synthetic corpus and reference observation](corpus/README.md), and the
[reference DLL crosswalk](docs/reference-dll-crosswalk.csv).

Windows was exercised locally, including native launch and egui workflow
automation. The existing CI matrix runs builds, Clippy and tests (including
the egui workflows) on Windows, macOS and Linux. macOS/Linux execution and
native desktop smoke checks remain to be confirmed in CI/on those systems.

Mathcad Prime 12.0.1.0 is an initial behavioral reference. PTC licensing and
legacy-only interfaces are out of scope. Original binaries, IL, restricted
fixtures and private worksheets must not be committed.
