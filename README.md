# Ham Monorepo

This repository is a single workspace for ham radio projects, experiments, and reusable libraries.

Expect many independent projects and libraries to live in different subdirectories. Some may be quick experiments, while others may grow into reusable crates, hardware tools, signal-processing utilities, firmware support code, or complete applications.

## Repository Layout

```text
.
├── Cargo.toml
├── crates/
│   └── ham-units-example/
└── projects/
    └── cargo-example/
```

- `crates/` contains reusable Rust libraries intended to be shared by multiple projects.
- `projects/` contains applications, experiments, prototypes, and project-specific binaries.
- A subdirectory may contain its own documentation, tests, examples, and notes.

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
