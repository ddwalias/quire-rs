# quire-rs

Quire is a cross-platform engineering worksheet application in early development.
The goal is a Rust application that runs on Windows, macOS and Linux and can
work with Mathcad Prime worksheets through independently implemented behavior.

This is a scaffold: the application opens an in-memory worksheet and can add
and edit text regions. **It cannot save files, import Mathcad worksheets or
evaluate equations yet.** Do not use it for work you need to retain.

## Run

Install the stable [Rust toolchain](https://rustup.rs/), then run:

```sh
cargo run -p quire
```

On Linux, the desktop build also needs development libraries for the selected
windowing backends, including X11, Wayland and xkbcommon. The CI workflow lists
the packages installed on Ubuntu.

## Develop

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

`quire-core` holds UI-independent worksheet data; `quire` is the desktop shell.
We will split packages when implementation shows a useful boundary, not to
match Mathcad's DLL count. See [architecture](docs/architecture.md),
[quality strategy](docs/quality.md), and the [reference DLL crosswalk](docs/reference-dll-crosswalk.csv).

Mathcad Prime 12.0.1.0 is an initial behavioral reference. PTC licensing and
legacy-only interfaces are out of scope. Original binaries, IL, restricted
fixtures and private worksheets must not be committed.
