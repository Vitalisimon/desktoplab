import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";

const vendorRoot = "third_party/glib-0.18.5-desktoplab";

test("Tauri uses the reviewed local glib UB backport", () => {
  const manifest = readFileSync("apps/desktop/src-tauri/Cargo.toml", "utf8");
  assert.match(
    manifest,
    /\[patch\.crates-io\][\s\S]*glib\s*=\s*\{\s*path\s*=\s*"\.\.\/\.\.\/\.\.\/third_party\/glib-0\.18\.5-desktoplab"\s*\}/,
  );

  const metadata = JSON.parse(readFileSync(`${vendorRoot}/DESKTOPLAB_BACKPORT.json`, "utf8"));
  assert.deepEqual(metadata, {
    package: "glib",
    version: "0.18.5",
    cratesIoChecksum: "233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5",
    advisory: "RUSTSEC-2024-0429",
    upstreamFixCommit: "05dff0ee696f9bcd8617cd48c4b812d046d440cb",
    patchedFile: "src/variant_iter.rs",
    patchedFileSha256: metadata.patchedFileSha256,
  });

  const source = readFileSync(`${vendorRoot}/${metadata.patchedFile}`, "utf8");
  assert.equal(createHash("sha256").update(source).digest("hex"), metadata.patchedFileSha256);
  assert.match(source, /let mut p: \*mut libc::c_char = std::ptr::null_mut\(\);/);
  assert.match(source, /g_variant_get_child\([\s\S]*?&mut p,/);
  assert.doesNotMatch(source, /let p: \*mut libc::c_char = std::ptr::null_mut\(\);/);
  assert.doesNotMatch(source, /g_variant_get_child\([\s\S]*?\n\s*&p,/);
});

test("the Tauri lockfile no longer resolves glib from crates.io", () => {
  const lockfile = readFileSync("apps/desktop/src-tauri/Cargo.lock", "utf8");
  const glibBlock = lockfile.match(/\[\[package\]\]\nname = "glib"\n[\s\S]*?(?=\n\[\[package\]\]|$)/)?.[0];
  assert.ok(glibBlock, "glib lockfile package is missing");
  assert.doesNotMatch(glibBlock, /^source = /m);
  assert.doesNotMatch(glibBlock, /^checksum = /m);
});
