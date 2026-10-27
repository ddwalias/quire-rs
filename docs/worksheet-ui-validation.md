# Worksheet canvas: reference research and validation

Date: 2026-10-27. This increment replaces the stacked form editor with a
page canvas, positioned regions, a grouped ribbon and typeset equations.

## Reference UI inspection

The installed **Mathcad Prime 12.0.1.0** UI assemblies were inspected with
ILSpyCmd 11.1.0.9782. This included selected managed classes and compiled
XAML (BAML) from the **en-US satellite resource DLLs**. This is deeper than
the earlier metadata-only DLL crosswalk. Raw decompilation and extracted
resources are confined to ignored `.cache/reference-ui/`; no original code,
XAML, icons or binaries were added to Git.

| Reference evidence | Observed structure | Quire implementation |
|---|---|---|
| `Spirit.SpiritMainWindow`, `spiritmainwindow.baml` | Ribbon window, docked document area, document tabs, bottom status/zoom controls | Grouped Math/Document/Calculation ribbon, one document strip, bottom status/zoom |
| `Spirit.Controls.ScrollViewerGrid` | Worksheet scroll host with a separate surrounding background | Independent page scroll/zoom transform on a dark surround |
| `worksheetcontrol.baml` | Separate visible page layer, positioned region canvas, page breaks and header/footer presenters | Page backgrounds and independently positioned text/math regions |
| `worksheetresources.baml` / `PageControlTemplate` | Page margins surround the body; page background and header/footer areas are separate | Plain white A4-like pages; document title remains in the document strip |
| `DirectRenderingAlignmentGridBorder`, `PageCanvasTemplate` | Pale blue major/minor line grid, configurable visibility and cell size | Optional blue line grid; Quire spacing and dimensions |
| `pagebodycanvas.baml` | Insertion crosshair and canvas items | Click-to-place crosshair; typing starts math at that position |
| `WorksheetControlContent` / `WhiteboardItemPositionComparer` / `PositionComparer` | Region coordinates, move commands, selection, location comparison by Y then X | Persistent page/X/Y positions; drag, nudge, move-up, deletion and reading order |
| `VisualFractionOperatorNode`, `VisualSuperscriptBinaryNode<T>` | Separate operand layout, fraction bar, baseline alignment and script font scaling | Independently implemented fraction/power layout over Quire's shared expression tree |

Original main assembly SHA-256 values, identifying the inspected installation:

- `MathcadPrime.dll`: `ae46024b823c32629838ffc0caae7a9141fb60597afc2c849cbc07cdfeb5e0b8`
- `Ptc.Controls.Worksheet.dll`: `f621eaf2c9706b7e5159481cd8f354862fe14f66c781b5b27505681f956fe498`
- `Ptc.Controls.Whiteboard.dll`: `c22ecb6a4207e0db9750becfb6f87b0ae75f9b1f9293f859a84671a5d5239324`
- `Ptc.Controls.EquationControl.dll`: `10a52c83ac41c77cb8265036ab84bc22d6bb160673b55c848bd3a58a0f8ea678`

These observations establish composition, not complete UI or behavioral
parity. Some decompiled methods had unresolved WPF type annotations. No
licensing behavior was implemented. Original ribbon libraries and assets are
not dependencies. Exact page defaults, all editor commands, collision rules,
printing, selection adorners and structural math cursor behavior remain
unverified. Quire deliberately uses stable array order for exact position
ties; it does not copy the reference comparer's tie handling.

## Implemented behavior

The portable core now owns a parsed expression tree, used by both the
evaluator and the UI typesetter. It retains explicit parentheses and numeric
source lexemes. Unsupported syntax displays as source with a clear error.
The renderer has no independent expression grammar.

The canvas has plain pages, a line grid, selection outlines, move handles,
text-width handles, mathematical fractions and superscripts, italic
identifiers, upright units, and inline evaluated results. Click the page and
type to start math; double-click a region or choose Edit for its source.
Editing overlays its source field without moving other regions. Long text
editors scroll; long previews indicate clipped content rather than delete it.

Regions calculate in page/Y/X order. Dragging or nudging across an earlier
definition changes the calculation order and can produce an undefined-name
error. Zoom changes only the view, not stored coordinates. The math editor
still edits source; it is not a structural equation editor.

Quire JSON v2 stores page positions and widths. V1 input migrates in its
existing order, retaining title, IDs and exact source. V2 rejects invalid
geometry and array order that contradicts visual order, so loading cannot
silently rearrange calculations. Unknown kinds/fields/versions remain errors.
Temporary-file replacement and failed-save preservation checks still pass.

## Validation

Local platform: **Windows 11 Enterprise x64**, Intel i7-1260P, Rust/Cargo
1.99.1. macOS and Linux were **not run locally**; their builds, lints and
portable UI workflows remain for the existing three-OS CI matrix. Native
macOS/Linux smoke checks, screen readers, IME and font/DPI variations remain
unrun. The declared Rust 1.95 minimum was not separately tested.

Commands:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p quire-core --release --example baseline --locked
git diff --check
```

The 33 tests comprise 13 desktop/typesetting tests and 20 core tests. They
cover the previous arithmetic/error/file workflows plus v1 migration,
multi-page positions, invalid/unknown layout data, reading-order changes,
typeset fraction bars and superscript placement, click-to-place and typing,
dragging, text resizing, zoom without document mutation, operator insertion
and keyboard focus, and positioned save/reopen. Tests send actual egui
pointer/keyboard events to the production UI and use a real temporary
filesystem; calculations and storage are not mocked.

Native Windows preview was exercised through UI Automation:

1. Open the authored example, select the first definition, and open Edit.
2. Change `a := 25 cm` to `a := 50 cm` and observe `150 cm` in the dependent
   region (and `0.75` / `2.25 m^2` in the ratio/area regions).
3. Save as ignored `.cache/native-canvas.quire.json`, create New, reopen it,
   and verify v2 positions and the `150 cm` result.
4. Inspect the native rendering at 75% zoom: ribbon groups, plain page, line
   grid, serif math, fractions, powers and inline results.

An older Quire executable was already running, so a separate ignored preview
manifest reused the production `main.rs` and `quire` library. The existing
window was left intact. Local screenshots remain in `.cache/`.

The authored fixture `corpus/canvas-example.quire.json` has SHA-256 (LF bytes)
`21492fb224a824c052071568d02cdd77a32c3fc73115bd0a6f0fbc6a33311d81`.
It contains no original Mathcad package data. Its ratio and area expectations
are independent arithmetic checks, not newly captured Mathcad results. The
previous limited `1.25 m` observation remains documented in the corpus record.

One release core baseline run with the same 1000-region workload gave first
calculation 1.2818 ms, edit/recalculation 1.4008 ms, mean of 100 recalculations
1.403416 ms, save/sync 13.9841 ms and reopen 0.8591 ms. This now includes
expression-tree construction and v2 layout serialization. These timings are
observations, not a controlled comparison or a performance budget.

## Remaining limits

No Mathcad file import/export, full structural equation editing, document-wide
undo/redo, multiple open documents, native file browsers, configurable paper
size or collision avoidance. Page dimensions, source-edit fields, supported
operators/units, error semantics and mathematical spacing are Quire behavior.
No pixel-identical rendering or Mathcad editing compatibility is claimed.

The bundled [STIX Two Math font](../apps/quire/assets/fonts/README.md) supplies
portable glyphs; platform-specific or PTC font/resource files are not bundled.
