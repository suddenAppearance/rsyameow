# Vendored dependencies

## `xattr`

A copy of [`xattr` 0.2.3](https://crates.io/crates/xattr/0.2.3), patched so that
`ENOATTR` maps to `ENODATA` on Linux and Android. Modern `libc` releases no
longer expose `ENOATTR` on these platforms, which breaks the unmodified crate.

`gpui 0.2.2` depends on `gpui_http_client` → `zed-async-tar` → `xattr 0.2.3`, so
the patch is applied through `[patch.crates-io]` in the workspace manifest.

The original MIT/Apache-2.0 licenses are preserved in this directory.
