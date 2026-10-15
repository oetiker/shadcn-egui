# Web gallery

## Goal

The gallery example (`crates/shadcn-egui/examples/gallery.rs`) shows every
component under a live preset picker, but only after a reader has installed
Rust and cloned the repository. This spec builds the same gallery for the
browser and publishes it on GitHub Pages at
`https://oetiker.github.io/shadcn-egui/`, so a reader who finds the crate on
crates.io or GitHub can try every style, palette, font, icon set and preset
without installing anything.

Requirements from the maintainer:

- The gallery carries every font and every icon set, like the desktop
  gallery. A slow first load is accepted.
- A loading page with a real progress bar covers the download.
- Pages updates on each release, so the gallery matches the version on
  crates.io. It can also be started by hand.

## Starting point

- The library compiles for `wasm32-unknown-unknown` with `--all-features`
  as it is. The only `std::thread` and `std::fs` calls are in tests. Building
  a style already fits the 1 MB wasm main-thread stack
  (`every_exported_style_builds_on_a_small_stack`).
- Only `main()` of the gallery example is desktop specific;
  `reference::gallery_ui` and `GalleryState` do the work.
- Fonts and icons are compiled into the binary: about 16 MB of TTF data and
  11 MB of icon data. The `.wasm` file will be about 30 MB; with the gzip
  compression GitHub Pages applies, the transfer is expected at 12 to 15 MB.

## Gallery code

`gallery.rs` gets two `main` functions:

- `#[cfg(not(target_arch = "wasm32"))]`: the current `main`, unchanged.
- `#[cfg(target_arch = "wasm32")]`: reads `preset` and `mode` from the query
  string of `window.location` (`?preset=<code>&mode=dark`), with the same
  meaning and the same fallback as the desktop arguments. An invalid preset
  code logs a warning to the browser console and uses `Preset::default()`.
  It starts `eframe::WebRunner` on `<canvas id="gallery">`. When `start`
  returns, it calls the page's `window.galleryStarted()`; when `start`
  fails, it calls `window.galleryFailed(<message>)` (see "Loading page").

New dev-dependencies, under `[target.'cfg(target_arch = "wasm32")'.dev-dependencies]`:
`wasm-bindgen`, `wasm-bindgen-futures`, `web-sys` (with the features the
entry point needs). Dev-dependencies do not reach crate users, and
`crates/shadcn-egui` packages only `src/**`, so the published crate does not
change.

## Web files

`web/index.html` at the repository root, written by hand. It holds the
canvas, the loading card and the loader script. Two placeholders are filled
in by the build script: `__WASM_SIZE__` (the size of the `.wasm` file in
bytes) and `__VERSION__` (the workspace version).

## Build script

`scripts/build-web-gallery.sh <out-dir>`, used locally and in CI:

1. `cargo build --release --jobs 4 --target wasm32-unknown-unknown --example gallery --features fonts-all,icons-all`.
2. Reads the `wasm-bindgen` version from `Cargo.lock` and compares it with
   `wasm-bindgen --version`. A missing tool or a different version stops the
   script with a message naming the version to install. A mismatched CLI
   fails later with an error that does not name the cause.
3. `wasm-bindgen --target web --no-typescript --out-dir <out-dir>` on the
   built example.
4. Copies `web/index.html` to `<out-dir>` and replaces the two placeholders.

`wasm-opt` is not part of this version. Most of the file is embedded font and
icon data, which it cannot shrink; it can be added if the code part turns out
large.

Local trial: run the script, then `python3 -m http.server` in `<out-dir>`.

## Loading page

- A card in the middle of the page, in the colours of the default preset
  (background, card, border, foreground, muted foreground, primary for the
  bar). Light or dark follows `?mode=` when present, otherwise
  `prefers-color-scheme`. The text uses the system font stack; Inter exists
  only inside the `.wasm` file.
- Text: `shadcn-egui <version> gallery`, then a progress bar and
  `42 % · 6.2 / 13.8 MB`.
- The script fetches the `.wasm` file with `fetch` and reads the body stream,
  counting bytes against `__WASM_SIZE__`. `Content-Length` cannot serve: with
  gzip it gives the compressed size while the stream yields uncompressed
  bytes. The collected bytes go to the wasm-bindgen `init` as a `Response`
  with the type `application/wasm`.
- After the download the text changes to `Starting…`. `galleryStarted()`
  removes the card.

Errors replace the progress bar with a message and a `Try again` button that
reloads the page:

- the fetch fails or returns an HTTP error status;
- the browser has no `WebAssembly` or no WebGL2 context;
- `galleryFailed(<message>)` is called.

A Rust panic after the start goes to the browser console, as eframe does by
default.

## CI build

`ci-local.yml` gets the job `web-gallery` ("Web gallery build"), following the
repo-infra contract for that file (`workflow_call` only, `timeout-minutes`,
`ref` checked out, `contents: read`, no job-level conditions):

1. Installs the stable toolchain with the `wasm32-unknown-unknown` target.
2. Downloads the prebuilt `wasm-bindgen` CLI for the version in `Cargo.lock`
   from the wasm-bindgen GitHub releases.
3. `cargo clippy --target wasm32-unknown-unknown --example gallery --features fonts-all,icons-all -- -D warnings`.
   The workspace clippy runs build for the host, where the wasm entry point is
   compiled out; this is its only lint.
4. `scripts/build-web-gallery.sh site`.
5. Uploads `site` as the artifact `web-gallery`, kept 7 days, so a reviewer
   can download and serve a PR's gallery.

## Pages deploy

New project-owned workflow `.github/workflows/pages.yml`.

Triggers:

- `workflow_run` on `Publish release (automatic)`, type `completed`. The
  release creates its tag and GitHub release with `GITHUB_TOKEN`, and events
  made with that token start no workflow, so `push: tags` or `release`
  triggers would never fire; `workflow_run` does.
- `workflow_dispatch` with the optional string input `ref`: a tag or branch.
  Empty takes the newest release (`gh release view --json tagName`).

Job `build` (`contents: read`):

1. For `workflow_run`: stops early with a note in the job summary when the
   run's conclusion is not `success`, or when its `head_sha` carries no `v*`
   tag. "Publish release" runs on every push to main that touches
   `CHANGES.md`, most of which are ordinary merges.
2. Checks out the resolved ref.
3. Fails with a clear message when `scripts/build-web-gallery.sh` is missing
   at that ref (releases before 0.5.0).
4. Same toolchain and CLI setup as the CI job, then the build script.
5. `actions/upload-pages-artifact`.

Job `deploy` (`needs: build`, runs when build sets its output `deploy` to `true`):
`actions/deploy-pages`, environment `github-pages`, permissions
`pages: write` and `id-token: write`. No other job gets these permissions.

One-time repository setting, done by the maintainer: Pages source
"GitHub Actions" (`gh api` command given in the PR).

A Pages failure leaves the release untouched: the crates and the GitHub
release are finished before `workflow_run` fires.

## Documentation

- `README.md`: a link "Try the gallery in your browser" near the top.
- `CHANGES.md`: an Added entry under [Unreleased].
- `RELEASING.md`: Pages deploys after each release; how to start it by hand.
- `CLAUDE.md`: `web/` and `scripts/build-web-gallery.sh` under Layout; the
  wasm clippy command under Gates.

## Testing

- No automated browser test. CI proves the web build compiles, is clippy
  clean and produces the site.
- Manual check before the PR, on a local build in Chrome: the progress bar
  advances and ends at 100 %; the gallery starts and the card disappears;
  `?preset=<code>&mode=dark` applies; a site without the `.wasm` file shows
  the error card.
- No component changes, so no kittest snapshot changes.

## Out of scope

- `wasm-opt` and other size work, lazy font or icon loading.
- A page that shows a Rust panic.
- A gallery for unreleased `main` on Pages (a manual run with `ref: main`
  does it once).
