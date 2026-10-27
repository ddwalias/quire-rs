# Worksheet fixtures

`lengths.quire.json` is an authored, synthetic Quire v1 worksheet containing
text, two definitions, and length evaluations in metres and centimetres.
It is safe to use in CI and contains no Mathcad package data. Its semantic
expectations are tested in `quire-core/tests/worksheet.rs`.

Case ID: `lengths-v1`. Origin: authored for this project, 2026-10-27.
Allowed handling: source control, editing and CI replay of this synthetic file
and the independently reproducible arithmetic below. No raw reference package,
binary, screenshot, IL or private document is included.

## Limited reference observation

On 2026-10-27, the locally installed **PTC Mathcad Express Prime 12.0.1.0**
was queried through its installed public automation interface (`GetVersion`
returned `12.0.1.0`). A new scratch worksheet was populated through the desktop
editor with explicit unit-labelled inputs:

```text
a := 25 * cm
b := a + 1 * m
b =
```

Observed display: **1.25 m**. Quire calculates an SI value of **1.25**, length
dimension **1**, and displays **1.25 m** for `b -> m`. This is an exact match
for this binary-exact value and the displayed unit. The unit dimension is
inferred from the displayed `m`, not extracted from Mathcad internals.

Settings observed in the reference UI: **Unit System: SI**, automatic
calculation enabled, ORIGIN 0, TOL 10^-3, CTOL 10^-3, English UI, default new
worksheet and default result formatting (precision setting not independently
inspected). Windows 11 x64; explicit decimal integers avoid locale ambiguity.
The Express title/status and settings were visually inspected. Raw captures
stay in ignored `.cache/reference/` and must not be committed.

The fixture also checks `b -> cm = 125 cm` and upstream editing to 50 cm,
which gives 1.5 m / 150 cm. These are independent arithmetic expectations;
**those conversions and edits were not separately observed in Mathcad**.
This single observation is not evidence of parser, error, rounding, full
numeric, or file compatibility. Mathcad document import and preservation
remain unverified and unsupported; no real `.mcdx` fixtures are included.

The fixture SHA-256 is recorded in the validation report. Recompute it after
an intentional fixture edit and explain any expected-value changes.

Keep private worksheets, original binaries and restricted reference material
out of Git. `corpus/private/` and `.mirror-cache/` are ignored for local work.
Record the origin, reference version, settings and comparison method for each
fixture before using it as a compatibility test. See `docs/quality.md`.
