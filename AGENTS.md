# Working on quire-rs

- Target Windows, macOS and Linux. Keep the worksheet and calculation core independent of GUI and platform-specific APIs.
- Read `docs/architecture.md` and `docs/quality.md` for the current direction. The DLL crosswalk records research and proposed ownership, not implemented compatibility.
- Develop complete user-facing features incrementally. Split modules into packages when a real dependency or testing boundary warrants it.
- Prefer Rust's standard library or suitable open-source dependencies for generic functionality. Keep Mathcad-specific behavior explicit and validate it against observed reference results where possible.
- Do not commit original Mathcad binaries, decompiled source, IL, restricted reference outputs or private worksheets. Put local reference data in ignored directories.
- PTC licensing and legacy-only interfaces are outside the current scope. Platform-specific compatibility adapters must remain optional.
- Use meaningful tests appropriate to the change. For numerical behavior and file handling, check edge cases, reference behavior and preservation. Never claim unsupported features or parity from scaffolding alone.
- Before finishing a code change, run appropriate formatting, lint and test checks; report platform checks that were not run.
