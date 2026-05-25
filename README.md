# Ham Monorepo

This repository is a single workspace for ham radio projects, experiments, and reusable libraries.

It currently includes reusable Rust crates for audio I/O, rig control, and FT8/FT4/FT2 decode/encode work, plus runnable projects for live weak-signal operation and decoder regression tracking.

## Repository Layout

```text
.
├── Cargo.toml
├── crates/
│   ├── audiolib/
│   ├── ft8-decoder/
│   ├── ham-units-example/
│   └── rigctl/
└── projects/
    ├── cargo-example/
    ├── ft8-regr/
    └── ft8op/
```

- `crates/` contains reusable Rust libraries intended to be shared by multiple projects.
- `projects/` contains applications, experiments, prototypes, and project-specific binaries.
- A subdirectory may contain its own documentation, tests, examples, and notes.

## Current Components

- `crates/ft8-decoder` is the FT8/FT4/FT2 decoder, encoder, and CLI harness. See [crates/ft8-decoder/README.md](crates/ft8-decoder/README.md).
- `crates/rigctl` is shared radio control support for K3S and mcHF-style rigs.
- `crates/audiolib` is shared platform-gated audio capture/playback support.
- `projects/ft8op` is the live FT8/FT4 operating app with web UI, queueing, QSO automation, rig control, and logging. See [projects/ft8op/README.md](projects/ft8op/README.md).
- `projects/ft8-regr` is the WSJT-X/Rust decoder regression and reporting project. See [projects/ft8-regr/README.md](projects/ft8-regr/README.md).
- `projects/ft8-regr/golden` contains curated, checked-in regression snapshots for static viewing. See [projects/ft8-regr/golden/README.md](projects/ft8-regr/golden/README.md).

## Cargo Workspace Guidelines

Most Rust code in this repository should be part of the top-level Cargo workspace. Add each Rust crate to the root `Cargo.toml` under `workspace.members`.

Use a workspace member for each independently buildable crate:

- Put shared libraries under `crates/<crate-name>`.
- Put runnable tools, demos, experiments, and applications under `projects/<project-name>`.
- Prefer small, focused library crates when behavior is useful across more than one project.
- Keep project-specific code inside that project until another project actually needs it.

Build and test from the repository root when possible:

```sh
cargo build --workspace
cargo test --workspace
```

Run one project by package name:

```sh
cargo run -p cargo-example
cargo run -p ft8op
```

## Sharing Code Between Subprojects

One subproject should import another through Cargo path dependencies. For example, a project under `projects/cargo-example` can depend on a reusable library under `crates/ham-units-example` like this:

```toml
[dependencies]
ham-units-example = { path = "../../crates/ham-units-example" }
```

Then Rust code can import it normally:

```rust
use ham_units_example::mhz_to_hz;
```

Prefer path dependencies inside this monorepo instead of publishing internal crates just to share code locally. If a crate becomes generally useful outside the repo, it can still be published later without changing how local projects consume it.

## Example

The included `cargo-example` project demonstrates the intended pattern:

- `crates/ham-units-example` is a reusable library crate.
- `projects/cargo-example` is a binary crate.
- `cargo-example` imports `ham-units-example` with a relative Cargo path dependency.

Try it with:

```sh
cargo run -p cargo-example
```

## Generated Outputs

Generated regression downloads, sample caches, reports, logs, and temporary files are ignored. In particular, `projects/ft8-regr/artifacts/` is scratch output created by the regression tools, while `projects/ft8-regr/golden/` is the curated checked-in archive for selected published snapshots.
