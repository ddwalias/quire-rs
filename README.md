# quire-rs

Quire is a cross-platform engineering worksheet application in early development.
The goal is a Rust application that runs on Windows, macOS and Linux and can
work with Mathcad Prime worksheets through independently implemented behavior.

The first worksheet slice supports editable text and math regions, ordered
variable definitions, real arithmetic, length units, automatic recalculation,
and save/reopen in **Quire's `.quire.json` format, version 1**. Math is edited
as a single line of source, with a result or error below it.

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

Click **Add text** for notes and **Add math** for each of these lines:

```text
a := 25 cm
b := a + 1 m
b -> m
b -> cm
```

The last two results are `1.25 m` and `125 cm`. Edit the first definition to
`a := 50 cm`: they become `1.5 m` and `150 cm`. Definitions apply only to later
regions; a redefinition affects subsequent uses. An invalid definition blocks
subsequent uses of that name instead of reusing its old value. **Move up**
changes worksheet order; **Delete** removes a region.

Enter a path ending in `.quire.json` in **File**, then click **Save** or
**Open**. Paths can be absolute or relative to the directory from which Quire
was started. To try the included example, open `corpus/lengths.quire.json`.
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
  region, 2048 tokens and 128 recursive parser levels. Invalid/unsupported math
  source remains editable and is preserved by save/reopen.

The JSON envelope identifies `quire-worksheet`, version `1`. It stores title,
ordered text/math source and stable region IDs; results are recalculated on
open. Unknown fields, versions or region kinds, malformed JSON and duplicate
IDs are rejected so a later save cannot silently lose unrecognized content.
Save serializes and validates first, writes and syncs a temporary file beside
the destination, then replaces it. Failed writes leave the original intact.
Symbolic-link destinations and read-only files are refused. Directory-entry
durability across sudden power loss is not guaranteed on every filesystem.

This slice has no 2D equation layout, functions, arrays, symbolic calculation,
other base dimensions, custom units, printing or document-wide undo/redo.
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
[synthetic corpus and reference observation](corpus/README.md), and the
[reference DLL crosswalk](docs/reference-dll-crosswalk.csv).

Windows was exercised locally, including native launch and egui workflow
automation. The existing CI matrix runs builds, Clippy and tests (including
the egui workflows) on Windows, macOS and Linux. macOS/Linux execution and
native desktop smoke checks remain to be confirmed in CI/on those systems.

Mathcad Prime 12.0.1.0 is an initial behavioral reference. PTC licensing and
legacy-only interfaces are out of scope. Original binaries, IL, restricted
fixtures and private worksheets must not be committed.
