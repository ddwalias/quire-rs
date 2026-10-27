# Proposed idiomatic Rust workspace, based on the Mathcad DLL scan

Status: recommendation for review, 2026-10-27. This supersedes the earlier proposal to create a candidate crate for each included Mathcad assembly. The complete [per-file crosswalk](reference-dll-crosswalk.csv) is part of this proposal. The initial workspace contains a small core and desktop shell; no Mathcad compatibility is claimed.

## Recommendation

Use **13 proposed workspace packages** when the full set of chosen capabilities warrants them; begin the first vertical slice with only the packages it actually needs. Organize each original assembly's functions as modules or adapters in these packages. Do not force a crate boundary for every DLL. The count is an architectural recommendation informed by the scan, not a Rust rule or a measured optimum; revisit boundaries after a working prototype.

Windows, macOS and Linux are first-class targets. PTC license/activation and explicitly legacy-only features are excluded per user direction. Generic implementation details use Rust std or established libraries where appropriate; feature behavior, file fidelity and platform-specific compatibility still require investigation and tests. The reference assembly identities remain in the crosswalk even when there is no corresponding Rust package.

## What was scanned

The installed Mathcad Prime 12.0.1.0 tree contained **404 DLLs and four EXEs**. Every file was read as PE: CLR/IL-only flags, machine field, regular imports, delay-load imports and named export counts were collected. Managed metadata was read for all 304 CLR files, including names, assembly references, declared types and declared methods. The remaining 104 files are native or resource PEs. Both scans completed for all 408 files without parse errors. The CSV has one row per actual installed path, including all 170 culture-satellite DLLs; the licensing-scope inventory was used as an initial classification, not as proof that it is merged into main.

The scan extracted **metadata, not behavior**. It did not reverse engineer every method body, follow every indirect or COM/dynamic load, discover all data files, validate every crate or run end-to-end feature tests. A managed IL-only PE's machine field does not by itself mean it can execute only on that architecture. Original binaries, decompiled source, IL and raw scan caches remain outside version control.

There are eight binaries with delay-load entries. Two important edges missed by the earlier ordinary-import map are `chartappole_sh/chartappole_sh.dll` -> `chartdir70.dll` and `efisymbolics_.dll` -> `efisymbolics-umgd.dll`. The other delay-loaded entries concern Intel MKL and Windows/native components. The CSV records ordinary and delay imports separately; it does not infer that every imported feature must exist in a Rust implementation.

## Proposed packages and their boundaries

| Cargo package | Kind | Owns | Why a crate instead of another module | Starting stage |
|---|---|---|---|---|
| `quire-app` | binary | Desktop composition, menus, lifecycle and wiring | Application entry point and platform distributions | First slice |
| `quire-document` | library | Worksheet model, region data, styles and stable IDs | Shared domain model without UI or evaluation implementation | First slice |
| `quire-format` | library | `.mcdx` and Quire package loading/saving, format versions, opaque content | Format I/O and dependencies isolated from model; also used by headless tools | First slice |
| `quire-syntax` | library | Expression AST, identifiers and syntax transformations | Shared by editor, document, evaluator and symbolics without UI | First slice |
| `quire-math` | library | Values, runtime units, function registry, numeric algorithms and compatible errors | One coherent value/quantity contract; numerical families start as modules | First slice |
| `quire-engine` | library | Ordered binding, evaluation, dependency analysis and invalidation | Headless evaluator used by app and tools, independent of UI toolkit | First slice |
| `quire-equation` | library | 2D math editing, layout, cursor and accessibility model | Reusable editor/layout behavior distinct from a particular desktop toolkit | First slice; merge into UI if spike shows no useful independent interface |
| `quire-ui` | library | Cross-platform worksheet canvas, regions, menus, interaction and rendering integration | UI toolkit dependencies stay outside document and engine | First slice |
| `quire-plot` | library | Plot model, sampling, axes and renderer-independent plot transforms | Reused by UI and headless rendering; isolate plotting dependencies | Later, or first slice only if plots required |
| `quire-symbolic` | library, optional | Backend protocol, assumptions, symbolic conversion and results | Optional external backend, separate dependencies and failure isolation | Optional |
| `quire-integration` | library | Portable file/data/spreadsheet/Python interfaces and adapters | Optional foreign dependencies kept outside core; make features independently selectable | Incremental |
| `quire-windows-interop` | library, optional | Windows COM/OLE and original user-function C ABI adapters | Windows-only build and native ABI must not leak into macOS/Linux | Optional |
| `quire-tools` | package with binaries | Headless CLI and compatibility/oracle tooling | Tool entry points; multiple binary targets can share one package | First slice for headless CLI; reference runner as needed |

Package names are functional, not copied DLL names. Each first-party package has its own `Cargo.toml`; modules beneath it are private unless a real public contract requires them. `quire-tools` can contain `quire` and `quire-compat` binary targets: a Cargo package can contain multiple binary crates. The Rust documentation distinguishes packages, crates and modules and describes Cargo workspace management: [packages and crates](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html), [workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html).

### Dependency direction

```text
quire-app        -> quire-ui, quire-engine, quire-format, quire-integration [optional]
quire-ui         -> quire-document, quire-syntax, quire-equation, quire-math, quire-plot [optional]
quire-equation   -> quire-syntax
quire-document   -> quire-syntax
quire-format     -> quire-document, quire-syntax
quire-engine     -> quire-document, quire-syntax, quire-math
quire-plot       -> quire-math
quire-symbolic   -> quire-syntax, quire-math, quire-engine [optional adapter]
quire-integration -> quire-document, quire-math, quire-engine [optional adapters]
quire-windows-interop -> quire-integration            [Windows only, optional]
quire-tools      -> quire-document, quire-format, quire-engine
```

`quire-app` selects optional backends and supplies interfaces at composition time. The engine defines the callbacks it needs for symbolic or external operations; adapters implement them, avoiding an engine -> adapter -> engine cycle. When implementing, check the complete graph for cycles; the diagram is a proposed design. The core and default features must compile without Windows APIs, a symbolic process, Python, Excel, or PTC binaries. UI rendering is a consumer of calculated snapshots, not an authority over the document.

### Modules, not crates, for related assemblies

Illustrative Rust layout, with no claim that every original function is fully covered:

```text
quire-math/src/
  value.rs  units.rs  functions/mod.rs
  functions/{bessel,statistics,fft,finance,fitting,signal,special}.rs
quire-ui/src/regions/
  {equation,text,include,solve,collapsible,spreadsheet}.rs
quire-symbolic/src/
  {lexer,parser,conversion,backend}.rs
```

For example `efibessel.dll` and `efibessel_.dll` map to `quire-math::functions::bessel`, while `Ptc.Controls.Text.dll` and `Ptc.Controls.Worksheet.dll` map to separate modules in `quire-ui`. Their source identity and individual status remain in the crosswalk. Units that must interoperate dynamically with values stay in `quire-math` initially, rather than introducing a `quire-units`/`quire-value` dependency boundary before we know the contract. A matrix/FFT engine can later become its own crate if a tested API, optional dependency set, compile-time bottleneck or independent consumers justify the split. Likewise, individual UI controls can become crates if the prototype proves separate consumers or platform constraints.

## Evidence behind the split

- `MathcadPrime.dll` references the worksheet, equation, calculation and controls assemblies. The worksheet assembly references equation controls, plots, text, whiteboard, include and solve regions. That supports grouping adjacent controls as UI modules rather than requiring a crate for each WPF DLL; it does not prove their APIs are identical.
- `Ptc.Calculation.dll` references both `Ptc.Calculation.Core` and `mplrun`, while calculator types are consumed by many UI assemblies. That supports a headless engine and shared math/value contract instead of one crate per original calculation assembly.
- `Ptc.EquationEditor.dll` has 491 declared metadata types and 3,313 declared methods in this scan. It is a substantial subsystem used by several UI assemblies; a separately testable equation-layout/editor boundary is justified as a starting hypothesis. It can be merged if it turns out tightly coupled to a chosen UI toolkit.
- The Mpl `Efi*` modules and corresponding `efi*_` implementations often represent the same function family at different original layers. Preserve individual Mathcad function semantics, but group the Rust interfaces and algorithms by feature in `quire-math` unless a real independent dependency emerges.
- `hashtable.dll` exposes checked maps, sets and inverse lookups. `std::collections::HashMap`/`HashSet` are appropriate primitives; verify equality, insertion and inverse semantics rather than creating a hash-table crate.
- The symbolic native implementation delay-loads a separate symbolic backend; it is a genuine optional integration boundary. `quire-symbolic` is justified even if the final backend is not FriCAS.
- `Ptc.Controls.ExcelComponent.dll` references `Ptc.OleInterop`; this warrants a Windows-only optional adapter. A portable spreadsheet reader is useful, but it is not a replacement for embedded Excel/OLE.
- Satellite DLLs and chart-bundle resources are resource data, not candidate code crates. Shared Creo licensing utilities are excluded for licensing use; independently verify any optional non-licensing consumers.

The full metadata behind these observations includes compiler-generated and third-party types; raw type/method totals are not estimates of implementation effort or feature completeness.

## External dependencies: recommendations, not parity claims

Choose versions and feature flags after cross-platform build, license and compatibility evaluation. Lock actual application versions in `Cargo.lock`.

| Function | Candidate | Important gap |
|---|---|---|
| Collections, threads, file primitives | Rust `std` | Mathcad key equality, ordering and error semantics still need compatibility adapters. |
| Worksheet container and XML | [`zip`](https://docs.rs/zip/latest/zip/), [`quick-xml`](https://docs.rs/quick-xml/latest/quick_xml/) | Neither is a complete Mathcad package implementation or guarantees lossless round-trip. |
| Cross-platform desktop | [`egui`/`eframe`](https://docs.rs/egui/latest/egui/) candidate | Prototype 2D math editing, IME, accessibility, large documents and printing on all three platforms. |
| Matrix and complex arithmetic | [`nalgebra`](https://docs.rs/nalgebra/latest/nalgebra/), [`num-complex`](https://docs.rs/num-complex/latest/num_complex/) | Indexing, units, shapes, error rules and numerical edge cases are Quire's responsibility. |
| Transforms and statistics | [`rustfft`](https://docs.rs/rustfft/latest/rustfft/), [`statrs`](https://docs.rs/statrs/latest/statrs/) | Normalization and distribution behavior need comparison; they do not cover every signal, statistics or special function. |
| Optimization | [`argmin`](https://docs.rs/argmin/latest/argmin/) candidate | Not established as a replacement for all solvers or Knitro constraints. |
| Spreadsheet data | [`calamine`](https://docs.rs/calamine/latest/calamine/), [`rust_xlsxwriter`](https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/) | Reader and new-file writer, respectively; neither provides OLE or general modification of existing XLSX files. |
| Plot drawing | [`plotters`](https://docs.rs/plotters/latest/plotters/) candidate | Interactive worksheet chart semantics and 3D remain to be designed. |
| Units | [`uom`](https://docs.rs/uom/latest/uom/) candidate | Test runtime/user-defined units and Mathcad dimensional behavior; may not fit the dynamic evaluator. |
| Windows compatibility | [`windows`](https://microsoft.github.io/windows-docs-rs/) in `quire-windows-interop` only | Bindings alone do not implement original COM/OLE or user-DLL ABI. |
| Symbolic engine | FriCAS process candidate | Integration, command semantics, assumptions, deployment and portability not established. |

## Cross-platform and compatibility rules

Run the portable core and desktop application in Windows, macOS and Linux CI. Compare numeric and document behavior across the three, including locale, file paths, Unicode, fonts, endianness/architecture where supported, and threading; define floating-point tolerances per function. Reference Mathcad execution, where available, runs on a Windows oracle; share permitted input cases and expected semantic results with the other platforms. GUI and rendering tests use platform-specific reference baselines where fonts or rasterization differ.

COM, OLE and loading Mathcad user DLLs are not portable APIs. If selected, `quire-windows-interop` handles those on Windows. Other platforms must open the document, preserve unsupported embedded content and report unavailable actions rather than discard content. Provide a separately designed portable automation and plugin interface; do not describe it as a drop-in replacement for the original Windows ABI. Licensing product code and licensing resources are excluded. Crates that require external native libraries must have supported builds for all intended platforms or stay optional with a documented fallback.

## Validation and next work

1. Confirm the supported Prime file versions, worksheet corpus, platform/architecture matrix and whether symbolic, OLE and original plugins are wanted.
2. Build a cross-platform proof of the core path: open, edit, evaluate with units, save a Quire document and compare selected results against Mathcad. Keep Prime files read-only until preservation is demonstrated.
3. Validate proposed crate seams with real types and dependency graphs. Test whether equation layout can remain UI-independent; merge or split crates based on actual coupling and use.
4. For each selected scientific family, compare function names, domains, units, errors, array conventions and numerical results against the reference. Generic crate choice alone does not establish coverage.
5. Extend the crosswalk from metadata-level proposals to per-feature compatibility evidence. Investigate native or dynamically loaded functionality where metadata is insufficient. Keep status unknown until observed or verified.

## File-by-file crosswalk

[reference-dll-crosswalk.csv](reference-dll-crosswalk.csv) has 408 rows: all 404 DLLs, including resource/satellite DLLs, and the four EXEs. Fields include PE/CLR classification, original managed assembly name and counts where available, native export count, managed references, ordinary native imports, delay imports, suggested treatment, Rust package/module and unresolved notes. It is a **planning proposal**, not a list of implemented functionality or a replacement for legal/licensing and runtime verification. Some DLLs span multiple crates; excluded licensing, compiler artifacts and third-party binaries have no parity crate.
