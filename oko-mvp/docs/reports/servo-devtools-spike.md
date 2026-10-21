# Servo devtools spike — control works, frames do not

Date: 2026-10-20. Pinned engine: Servo 0.7.0-aac43a3f3, aarch64 Linux.
No `DISPLAY`, no `WAYLAND_DISPLAY` for every run.

## Question

Can the pinned Servo drive a live web surface with no display server: navigate,
run script, and produce frames the display server can present?

## Verified

- `servoshell -z --devtools=127.0.0.1:7000 about:blank` binds and serves with
  no display server.
- The transport is **not** HTTP, WebSocket or a 4-byte binary length. It is a
  decimal length, a colon, then JSON:

      195:{"from":"root",...}

  A 4-byte length, a newline-delimited JSON, and `Content-Length` framing all
  fail with `packet length missing / not parsable`.
- Root greeting:

      {"from":"root","applicationType":"browser","traits":{...}}

- `{"to":"root","type":"listTabs"}` returns a `TabDescriptorActor`.
- `{"to":"<tab>","type":"getTarget"}` returns a `BrowsingContextActor` with
  `consoleActor`, `inspectorActor`, `reflowActor`, `styleSheetsActor`,
  `threadActor`, and `traits.navigation: true`.
- `{"to":"<tab>","type":"getWatcher"}` returns a `WatcherActor`.
- `ConsoleActor.evaluateJS` executes script and returns a result:
  `{"to":"<console>","type":"evaluateJS","text":"1+1"}` -> `result: 2.0`.
- `location.href = "data:..."` navigates the live session and the watcher
  emits `will-navigate`.

## Not available

- `BrowsingContextActor` rejects `navigate`, `reload`, `screenshot`, `capture`,
  `getScreenshot` and `snapshot` with `unrecognizedPacketType`.
- `TabDescriptorActor` rejects `navigate` and `reload` the same way.
- There is no screenshot/capture actor in the strings or in the actor graph.
- `servoshell -z -o frame.bmp <url>` writes **one** BMP at startup. After a
  devtools navigation the file size and SHA-256 do not change, and killing the
  process does not rewrite it. `-o` is a startup snapshot, not a frame sink.

## Consequence

The prebuilt devtools surface is a debugging/control protocol, not a rendering
surface. It can navigate and evaluate script, but it cannot give the display
server pixels. Do not design a frame pump on it.

Live web frames require one of:

1. A Servo embedder built with a rendering context the host can read
   (`surfaceless` EGL or a software/DRM backend), which is a custom build and a
   separate project;
2. a different engine/build that exposes an offscreen surface;
3. a controlled HTML/CSS/JS subset renderer, which is a browser project and is
   explicitly out of the current scope;
4. accepting one-shot snapshots: `oko-preview` renders a URL to a BMP and
   pushes it as a surface, with no interaction and no live state.

The current display server and input routing do not depend on this decision.
`oko-preview --socket` remains the working web-to-surface path.

## Reproduce

The spike scripts are in `/tmp/devtools_spike*.py`; the essential client is:

    data = json.dumps(message).encode()
    sock.sendall(str(len(data)).encode() + b":" + data)

Read a decimal length up to `:`, then exactly that many bytes of JSON.
