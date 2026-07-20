# DesktopLab glib backport

This directory vendors the crates.io `glib 0.18.5` package because Tauri 2's
Linux GTK3 stack cannot consume `glib 0.20`.

DesktopLab applies the upstream fix for `RUSTSEC-2024-0429` from gtk-rs commit
`05dff0ee696f9bcd8617cd48c4b812d046d440cb`. The only upstream source change is
in `src/variant_iter.rs`: the C out-parameter is mutable and passed as
`&mut p`, removing the undefined behavior described by the advisory.

The original MIT license and copyright files are retained. Do not modify this
vendor directory except through an explicit dependency upgrade or reviewed
security backport. `scripts/security/glib-backport.test.mjs` binds the Tauri
manifest, lockfile, upstream metadata and patched source hash.
