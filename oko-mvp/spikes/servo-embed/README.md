# servo-embed spike

A standalone probe for the published `servo` 0.7.0 crate. It creates a
`SoftwareRenderingContext`, loads a `data:` URL in a `WebView`, takes a
screenshot, and writes `frame.ppm`. No display server is involved.

This is a spike, not a production client. It is not a workspace member of
`oko-mvp`, so the default ỌKỌ build never depends on Servo (ADR 0009).

## Build

    sudo apt-get install -y zlib1g-dev
    cargo run --release

`zlib1g-dev` is needed because `freetype-sys` builds bundled libpng. SpiderMonkey
should come from the prebuilt `mozjs` archive; it does not need a source build
when the archive is available.

## CI

`.github/workflows/servo-spike.yml` is a manual `workflow_dispatch` job that
builds this spike on `ubuntu-24.04` and uploads `frame.ppm`. Use it instead of a
small container. The runner has more RAM than the 2.7 GB development host.

## Host requirement

The build is large. On the 2.7 GB development container, `cargo check` aborts
in `stylo 0.22.0` under a 2.4 GB address-space cap:

    memory allocation of 2654216 bytes failed
    error: could not compile `stylo` (lib)  (SIGABRT)

That is a host memory limit, not an API error. Budget at least 8 GB RAM for
the build and measure the real requirement on the target host. Do not run
Servo under `ulimit -v`: the prebuilt engine reserves address space and exits
101. Use a process timeout.

## What to do with the result

If `frame.ppm` appears with no `DISPLAY`/`WAYLAND_DISPLAY`, the API is proven.
The production client is a separate `oko-servo-embed` process that:

1. connects to `oko-display` as a surface client;
2. creates a surface at the viewport size;
3. calls `paint`/`read_to_image` on `notify_new_frame_ready` and commits frames;
4. maps `ServerMessage::Input` to Servo `InputEvent`s and calls
   `notify_input_event`.

See `docs/todo/08-servo-embedder.txt` and `docs/adr/0009-servo-embedder-process.md`.
