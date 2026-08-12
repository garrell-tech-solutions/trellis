# QA Procedure: Release binary is a single static executable

Covers: `features/release_binary.feature`

## Procedure
1. From a clean checkout of the workspace, run the project's release build
   for the musl target (e.g.
   `cargo build --release --target x86_64-unknown-linux-musl`).
2. List the produced binaries under the release output directory for that
   target.
3. Run the project's dynamic-linking inspection affordance against the
   produced binary (e.g. `ldd <binary>`).

## Expected Observable Outcomes
- Exactly one executable binary is produced under the release output
  directory for the musl target.
- The dynamic-linking inspection reports the binary as not a dynamic
  executable (e.g. `ldd` prints "not a dynamic executable").

## Independent of Implementation
This procedure only depends on the build command's output artifact and
`ldd`'s report on it — not on which crate contains `main`, crate layout, or
build script internals.
