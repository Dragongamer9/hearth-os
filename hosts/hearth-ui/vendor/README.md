# Vendored crates

- `softbuffer/`: softbuffer 0.4.8 from crates.io (MIT / Apache-2.0, its licences beside it), patched in one place:
  `src/backends/wayland/buffer.rs` `shm_format()`. With `HEARTH_SOFTBUFFER_ALPHA=1` its Wayland buffers are ARGB8888
  (premultiplied, as tiny-skia draws) instead of XRGB8888, so a window's transparent parts are see-through. Only
  hearth-pad-host (apps/pad) sets it, for the controller's overlays over Plasma (the dimmed Hearth menu, the hints, the
  keyboard's edges); every other app is drawn exactly as before. Used through `[patch.crates-io]` in ../Cargo.toml.
