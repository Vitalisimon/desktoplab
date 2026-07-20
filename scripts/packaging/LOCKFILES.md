# Cargo lockfile ownership

DesktopLab intentionally has two authoritative Cargo dependency graphs:

- `/Cargo.lock` belongs to the root Cargo workspace and is verified with
  `cargo test --locked --workspace`.
- `/apps/desktop/src-tauri/Cargo.lock` belongs to the standalone Tauri workspace
  declared by `/apps/desktop/src-tauri/Cargo.toml` and is consumed by
  `tauri build ... -- --locked`.

Package builds must not resolve or update either graph. The packaging lane runs
`verify-lockfiles-clean.sh` after the build and CI checks both locked graphs
before accepting package evidence. Dependency changes must update and commit
the lockfile owned by the changed manifest before packaging.
