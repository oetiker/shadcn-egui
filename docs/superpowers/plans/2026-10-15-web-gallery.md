# Web Gallery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the gallery example for the browser and publish it on GitHub Pages after each release, behind a loading page with a real progress bar.

**Architecture:** `gallery.rs` gets a second, wasm-only `main` that starts `eframe::WebRunner`; both `main`s share one function that turns the preset and mode strings into a `GalleryState`. A shell script builds the example for `wasm32-unknown-unknown`, runs the `wasm-bindgen` CLI pinned to the `Cargo.lock` version and fills two placeholders in a hand-written `web/index.html`. `ci-local.yml` builds the site on every PR; a new `pages.yml` builds it from the newest release and deploys it.

**Tech Stack:** Rust (eframe 0.34.3, wasm-bindgen 0.2.125, wasm-bindgen-futures 0.4.75, web-sys 0.3.102), Bash, plain HTML/CSS/JS module, GitHub Actions (`actions/upload-pages-artifact@v5`, `actions/deploy-pages@v5`).

**Spec:** `docs/superpowers/specs/2026-10-15-web-gallery-design.md`

## Global Constraints

- Every cargo command gets `--jobs 4`; every `cargo test` also `-- --test-threads 4`.
- No new runtime `[dependencies]`; the wasm crates are dev-dependencies under `[target.'cfg(target_arch = "wasm32")'.dev-dependencies]`.
- The `wasm-bindgen` CLI version must equal the `wasm-bindgen` version in `Cargo.lock`; scripts read it from there, never hard-code it.
- `ci-local.yml` jobs: `workflow_call` only, `timeout-minutes` on every job, `ref: ${{ inputs.ref }}` on every checkout, no more than `contents: read`, no job-level `if:`.
- Only the `deploy` job of `pages.yml` gets `pages: write` and `id-token: write`.
- The repository is public: no local or scratch paths in committed files.
- Prose follows the `repo-infra:writing-style` skill: no em dashes, "for example" instead of the Latin abbreviation, CHANGES entries name what a user sees.
- Local build output and the `wasm-bindgen` CLI go to a scratch directory outside the repository.

## Review Focus

- A preset code in the URL that does not decode (`?preset=zz`): the gallery starts with the default preset and the console shows a warning naming the code. Pinned by `initial_state` tests in Task 1.
- `index.html` opened before the build filled `__WASM_SIZE__`, or served by a server that sends no length: the progress text shows the MB loaded without a percent instead of `NaN %`. Pinned by the manual check in Task 2, step 7.
- `gallery.js` or `gallery_bg.wasm` missing or a 404: the card shows an error and `Try again`, never a frozen "Loading…". Pinned by Task 2, step 8.
- A "Publish release" run for an ordinary merge that touched `CHANGES.md`: `pages.yml` deploys nothing and says why in the job summary. Pinned by the `workflow_run` dry run in Task 4, step 3.
- A manual run of `pages.yml` for a ref older than 0.5.0 (no build script): the job fails with a message naming the ref. Pinned by Task 4, step 3.

---

## File structure

- Modify `crates/shadcn-egui/examples/gallery.rs`: shared `initial_state`, native `main`, wasm `main`, tests.
- Modify `crates/shadcn-egui/Cargo.toml`: wasm dev-dependencies, `test = true` on the gallery example.
- Create `web/index.html`: canvas, loading card, loader module.
- Create `scripts/install-wasm-bindgen.sh`: downloads the CLI matching `Cargo.lock` into a directory.
- Create `scripts/build-web-gallery.sh`: builds the site into an output directory.
- Modify `.github/workflows/ci-local.yml`: job `web-gallery`.
- Create `.github/workflows/pages.yml`: release-driven build and deploy.
- Modify `README.md`, `CHANGES.md`, `RELEASING.md`, `CLAUDE.md`.

---

### Task 1: Gallery entry point for the browser

**Files:**
- Modify: `crates/shadcn-egui/examples/gallery.rs` (whole file)
- Modify: `crates/shadcn-egui/Cargo.toml:199-210`

**Interfaces:**
- Produces: `fn initial_state(preset: Option<&str>, mode: Option<&str>) -> (GalleryState, Option<String>)`; the string is a warning to show the user.
- Produces: the wasm entry point calls the JS globals `window.galleryStarted()` and `window.galleryFailed(message: string)`, and draws on `<canvas id="gallery">`. Task 2 defines these in `web/index.html`.

- [ ] **Step 1: Enable tests for the gallery example**

In `crates/shadcn-egui/Cargo.toml`, change the gallery example entry to:

```toml
[[example]]
name = "gallery"
required-features = ["fonts-all", "icons-all"]
# The tests of initial_state run with cargo test --all-features.
test = true
```

- [ ] **Step 2: Write the failing tests**

Append to `crates/shadcn-egui/examples/gallery.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_given_is_the_default_preset_in_light_mode() {
        let (state, warning) = initial_state(None, None);
        assert_eq!(state.preset, Preset::default());
        assert_eq!(state.mode, Mode::Light);
        assert_eq!(warning, None);
    }

    #[test]
    fn a_valid_code_and_dark_apply() {
        let (state, warning) = initial_state(Some("b0"), Some("dark"));
        assert_eq!(state.preset, Preset::from_code("b0").expect("decodes"));
        assert_eq!(state.mode, Mode::Dark);
        assert_eq!(warning, None);
    }

    #[test]
    fn a_bad_code_warns_and_falls_back_to_the_default() {
        let (state, warning) = initial_state(Some("c0"), None);
        assert_eq!(state.preset, Preset::default());
        let warning = warning.expect("a warning");
        assert!(warning.contains("c0"), "{warning}");
    }

    #[test]
    fn any_mode_other_than_dark_is_light() {
        for mode in ["light", "DARK", ""] {
            assert_eq!(initial_state(None, Some(mode)).0.mode, Mode::Light, "{mode}");
        }
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --jobs 4 -p shadcn-egui --features fonts-all,icons-all --example gallery -- --test-threads 4`
Expected: compile error `cannot find function initial_state`.

- [ ] **Step 4: Write the implementation**

Replace everything above `#[cfg(test)]` in `gallery.rs` with:

```rust
//! Every component under a live preset picker:
//! `cargo run --example gallery --features fonts-all,icons-all [-- <preset code> [dark|light]]`.
//!
//! The same example runs in the browser: `scripts/build-web-gallery.sh`
//! builds it, and the page takes `?preset=<code>&mode=dark`.

use shadcn_egui::reference::{gallery_ui, GalleryState};
use shadcn_egui::{Mode, Preset};

struct GalleryApp {
    state: GalleryState,
}

impl eframe::App for GalleryApp {
    // gallery_ui applies the preset itself, only when a picker changed it.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        gallery_ui(ui, &mut self.state);
    }
}

/// The starting state from a preset code and a mode, as the desktop
/// arguments or the page's query string give them, plus a warning for the
/// user when the code does not decode.
fn initial_state(preset: Option<&str>, mode: Option<&str>) -> (GalleryState, Option<String>) {
    let mut warning = None;
    let preset = match preset {
        Some(code) => Preset::from_code(code).unwrap_or_else(|e| {
            warning = Some(format!("gallery: {code}: {e}; using the default preset"));
            Preset::default()
        }),
        None => Preset::default(),
    };
    let mode = if mode == Some("dark") {
        Mode::Dark
    } else {
        Mode::Light
    };
    (GalleryState::new(preset, mode), warning)
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (state, warning) = initial_state(
        args.get(1).map(String::as_str),
        args.get(2).map(String::as_str),
    );
    if let Some(warning) = warning {
        eprintln!("{warning}");
    }
    eframe::run_native(
        "shadcn-egui gallery",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 900.0]),
            ..Default::default()
        },
        Box::new(move |_cc| Ok(Box::new(GalleryApp { state }))),
    )
}

/// The page's loader script (`web/index.html`) defines both functions:
/// one removes the loading card, the other shows the message on it.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = galleryStarted)]
    fn gallery_started();
    #[wasm_bindgen(js_namespace = window, js_name = galleryFailed)]
    fn gallery_failed(message: &str);
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast as _;

    eframe::WebLogger::init(log::LevelFilter::Warn).ok();
    let window = web_sys::window().expect("a browser window");
    let query = window.location().search().unwrap_or_default();
    let params = web_sys::UrlSearchParams::new_with_str(&query).expect("a query string");
    let (state, warning) = initial_state(
        params.get("preset").as_deref(),
        params.get("mode").as_deref(),
    );
    if let Some(warning) = warning {
        log::warn!("{warning}");
    }
    let canvas = window
        .document()
        .and_then(|d| d.get_element_by_id("gallery"))
        .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        .expect("<canvas id=\"gallery\">");
    wasm_bindgen_futures::spawn_local(async move {
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |_cc| Ok(Box::new(GalleryApp { state }))),
            )
            .await;
        match started {
            Ok(()) => gallery_started(),
            Err(e) => gallery_failed(&e.as_string().unwrap_or_else(|| format!("{e:?}"))),
        }
    });
}
```

Note: `WebRunner` must outlive `start`; eframe keeps the app runner alive through the callbacks it installs on the canvas, so dropping the `WebRunner` handle after `start` is what eframe's own `web_demo` does. If the gallery freezes after the first frame in Step 6 of Task 2, keep the runner in a `thread_local!` or `Box::leak` it and note the reason in a comment.

- [ ] **Step 5: Add the wasm dev-dependencies**

In `crates/shadcn-egui/Cargo.toml`, after the `[dev-dependencies]` block (before the first `[[example]]`), add:

```toml
# The gallery example's browser entry point; scripts/build-web-gallery.sh
# runs the wasm-bindgen CLI of the version Cargo.lock pins for this crate.
[target.'cfg(target_arch = "wasm32")'.dev-dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = ["Document", "Element", "HtmlCanvasElement", "Location", "UrlSearchParams", "Window"] }
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test --jobs 4 -p shadcn-egui --features fonts-all,icons-all --example gallery -- --test-threads 4`
Expected: 4 passed.

Run: `git diff --stat Cargo.lock`
Expected: no change, or only new entries for crates already resolved for eframe's wasm target. Versions of `wasm-bindgen`, `wasm-bindgen-futures`, `web-sys` stay 0.2.125, 0.4.75, 0.3.102.

- [ ] **Step 7: Check the wasm build and lints**

Run: `rustup target list --installed | grep wasm32-unknown-unknown` (expected: present).
Run: `cargo clippy --locked --jobs 4 -p shadcn-egui --target wasm32-unknown-unknown --example gallery --features fonts-all,icons-all -- -D warnings`
Expected: no warnings.
Run: `cargo clippy --locked --jobs 4 --workspace --all-targets --all-features -- -D warnings` and `cargo fmt --all --check`
Expected: clean.

- [ ] **Step 8: Run the desktop gallery once**

Run: `cargo run --jobs 4 --example gallery --features fonts-all,icons-all -- c0` on a machine with a display, or skip and say so.
Expected: stderr shows `gallery: c0: … using the default preset` and the window opens.

- [ ] **Step 9: Commit**

```bash
git add crates/shadcn-egui/examples/gallery.rs crates/shadcn-egui/Cargo.toml Cargo.lock
git commit -m "gallery: browser entry point reading preset and mode from the URL"
```

---

### Task 2: Loading page and build scripts

**Files:**
- Create: `web/index.html`
- Create: `scripts/install-wasm-bindgen.sh`
- Create: `scripts/build-web-gallery.sh`

**Interfaces:**
- Consumes: from Task 1, `window.galleryStarted()`, `window.galleryFailed(message)`, `<canvas id="gallery">`; wasm-bindgen names its output after the example: `gallery.js`, `gallery_bg.wasm`.
- Produces: `scripts/install-wasm-bindgen.sh <dir>` puts the CLI at `<dir>/wasm-bindgen`; `scripts/build-web-gallery.sh <out-dir>` writes `<out-dir>/{index.html,gallery.js,gallery_bg.wasm}`. The build script uses `$WASM_BINDGEN` when set, else `wasm-bindgen` on `PATH`. Tasks 3 and 4 call both scripts.

- [ ] **Step 1: Write `scripts/install-wasm-bindgen.sh`**

```bash
#!/usr/bin/env bash
# Downloads the prebuilt wasm-bindgen CLI whose version matches the
# wasm-bindgen crate in Cargo.lock and puts it at <dir>/wasm-bindgen.
# A CLI of another version fails on the generated module with an error
# that does not name the version, so the version comes from the lock file
# that Dependabot updates.
set -euo pipefail

dir=${1:?usage: scripts/install-wasm-bindgen.sh <dir>}
root=$(cd "$(dirname "$0")/.." && pwd)
version=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/version = |"/, ""); print; exit }' "$root/Cargo.lock")
[ -n "$version" ] || { echo "install-wasm-bindgen: no wasm-bindgen in Cargo.lock" >&2; exit 1; }

name=wasm-bindgen-$version-x86_64-unknown-linux-musl
mkdir -p "$dir"
curl -fsSL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$version/$name.tar.gz" \
    | tar -xz -C "$dir" --strip-components 1 "$name/wasm-bindgen"
"$dir/wasm-bindgen" --version
```

- [ ] **Step 2: Write `scripts/build-web-gallery.sh`**

```bash
#!/usr/bin/env bash
# Builds the browser gallery into <out-dir>: the gallery example for
# wasm32-unknown-unknown, the wasm-bindgen output for it, and
# web/index.html with the wasm size and the version filled in.
set -euo pipefail

out=${1:?usage: scripts/build-web-gallery.sh <out-dir>}
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

bindgen=${WASM_BINDGEN:-wasm-bindgen}
want=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/version = |"/, ""); print; exit }' Cargo.lock)
have=$("$bindgen" --version 2>/dev/null | awk '{ print $2 }' || true)
if [ "$have" != "$want" ]; then
    echo "build-web-gallery: needs wasm-bindgen $want, found ${have:-none}." >&2
    echo "Install it with scripts/install-wasm-bindgen.sh <dir> and set WASM_BINDGEN=<dir>/wasm-bindgen." >&2
    exit 1
fi

version=$(awk -F'"' '/^\[workspace.package\]/ { p = 1 } p && /^version/ { print $2; exit }' Cargo.toml)

cargo build --locked --release --jobs 4 --target wasm32-unknown-unknown \
    --example gallery --features fonts-all,icons-all
target=$(cargo metadata --locked --format-version 1 --no-deps \
    | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')

mkdir -p "$out"
"$bindgen" --target web --no-typescript --out-dir "$out" \
    "$target/wasm32-unknown-unknown/release/examples/gallery.wasm"
size=$(( $(wc -c < "$out/gallery_bg.wasm") ))
sed -e "s/__WASM_SIZE__/$size/g" -e "s/__VERSION__/$version/g" web/index.html > "$out/index.html"
echo "build-web-gallery: $out, $size bytes of wasm, version $version"
```

Run: `chmod +x scripts/install-wasm-bindgen.sh scripts/build-web-gallery.sh`

- [ ] **Step 3: Check the version guard**

Run: `WASM_BINDGEN=/nonexistent scripts/build-web-gallery.sh "$SCRATCH/site"` where `$SCRATCH` is a scratch directory outside the repository.
Expected: exit 1 with `needs wasm-bindgen 0.2.125, found none.`

- [ ] **Step 4: Write `web/index.html`**

The colours are the shadcn neutral theme of the default preset (`Preset::default()`: nova, neutral), light and dark.

```html
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>shadcn-egui gallery</title>
<script>
  // ?mode=dark forces dark, any other ?mode= forces light (as in the
  // gallery); without it the page follows the browser setting.
  const mode = new URLSearchParams(location.search).get("mode");
  if (mode !== null) document.documentElement.dataset.mode = mode === "dark" ? "dark" : "light";
</script>
<style>
  :root {
    --background: oklch(1 0 0);
    --foreground: oklch(0.145 0 0);
    --card: oklch(1 0 0);
    --muted: oklch(0.97 0 0);
    --muted-foreground: oklch(0.556 0 0);
    --border: oklch(0.922 0 0);
    --primary: oklch(0.205 0 0);
    --primary-foreground: oklch(0.985 0 0);
    --destructive: oklch(0.577 0.245 27.325);
    --radius: 0.625rem;
  }
  @media (prefers-color-scheme: dark) {
    :root:not([data-mode="light"]) {
      --background: oklch(0.145 0 0);
      --foreground: oklch(0.985 0 0);
      --card: oklch(0.205 0 0);
      --muted: oklch(0.269 0 0);
      --muted-foreground: oklch(0.708 0 0);
      --border: oklch(1 0 0 / 10%);
      --primary: oklch(0.922 0 0);
      --primary-foreground: oklch(0.205 0 0);
      --destructive: oklch(0.704 0.191 22.216);
    }
  }
  :root[data-mode="dark"] {
    --background: oklch(0.145 0 0);
    --foreground: oklch(0.985 0 0);
    --card: oklch(0.205 0 0);
    --muted: oklch(0.269 0 0);
    --muted-foreground: oklch(0.708 0 0);
    --border: oklch(1 0 0 / 10%);
    --primary: oklch(0.922 0 0);
    --primary-foreground: oklch(0.205 0 0);
    --destructive: oklch(0.704 0.191 22.216);
  }
  html, body {
    margin: 0;
    height: 100%;
    overflow: hidden;
    background: var(--background);
    color: var(--foreground);
    font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  }
  #gallery { display: block; width: 100%; height: 100%; }
  #loader {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--background);
  }
  .card {
    box-sizing: border-box;
    width: min(22rem, calc(100% - 2rem));
    padding: 1.5rem;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: calc(var(--radius) + 4px);
    box-shadow: 0 1px 2px rgb(0 0 0 / 5%);
  }
  h1 { margin: 0 0 1rem; font-size: 1rem; font-weight: 600; }
  .track { height: 0.5rem; overflow: hidden; background: var(--muted); border-radius: 999px; }
  .bar { width: 0; height: 100%; background: var(--primary); transition: width 120ms linear; }
  #status {
    margin: 0.75rem 0 0;
    font-size: 0.875rem;
    color: var(--muted-foreground);
    font-variant-numeric: tabular-nums;
  }
  #status.error { color: var(--destructive); }
  #retry {
    margin-top: 1rem;
    padding: 0.5rem 1rem;
    font: inherit;
    font-size: 0.875rem;
    color: var(--primary-foreground);
    background: var(--primary);
    border: 0;
    border-radius: var(--radius);
    cursor: pointer;
  }
</style>
</head>
<body>
<canvas id="gallery"></canvas>
<div id="loader" role="status" aria-live="polite">
  <div class="card">
    <h1>shadcn-egui __VERSION__ gallery</h1>
    <div class="track" id="track"><div class="bar" id="bar"></div></div>
    <p id="status">Loading…</p>
    <button id="retry" type="button" hidden>Try again</button>
  </div>
</div>
<script type="module">
  // The build script replaces __WASM_SIZE__ with the size of the .wasm file.
  // Content-Length cannot serve: with gzip it is the compressed size, while
  // the stream below yields uncompressed bytes.
  const wasmSize = Number("__WASM_SIZE__");
  const bar = document.getElementById("bar");
  const status = document.getElementById("status");
  const retry = document.getElementById("retry");
  const mb = (bytes) => (bytes / 1e6).toFixed(1);

  function fail(message) {
    document.getElementById("track").hidden = true;
    status.textContent = message;
    status.classList.add("error");
    retry.hidden = false;
  }
  retry.addEventListener("click", () => location.reload());
  window.galleryStarted = () => document.getElementById("loader").remove();
  window.galleryFailed = (message) => fail(`The gallery could not start: ${message}`);

  function progress(loaded) {
    if (wasmSize > 0) {
      const percent = Math.min(100, Math.floor((loaded * 100) / wasmSize));
      bar.style.width = `${percent}%`;
      status.textContent = `${percent} % · ${mb(loaded)} / ${mb(wasmSize)} MB`;
    } else {
      status.textContent = `${mb(loaded)} MB`;
    }
  }

  async function download(url) {
    const response = await fetch(url);
    if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
    const reader = response.body.getReader();
    const chunks = [];
    let loaded = 0;
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
      loaded += value.length;
      progress(loaded);
    }
    return new Blob(chunks, { type: "application/wasm" });
  }

  async function run() {
    if (typeof WebAssembly !== "object") return fail("This browser does not support WebAssembly.");
    if (!document.createElement("canvas").getContext("webgl2")) {
      return fail("This browser does not support WebGL2.");
    }
    let init, wasm;
    try {
      ({ default: init } = await import("./gallery.js"));
      wasm = await download("./gallery_bg.wasm");
    } catch (e) {
      return fail(`The download failed: ${e.message}`);
    }
    bar.style.width = "100%";
    status.textContent = "Starting…";
    try {
      await init({ module_or_path: new Response(wasm, { headers: { "Content-Type": "application/wasm" } }) });
    } catch (e) {
      fail(`The gallery could not start: ${e}`);
    }
  }
  run();
</script>
</body>
</html>
```

- [ ] **Step 5: Build the site locally**

```bash
scripts/install-wasm-bindgen.sh "$SCRATCH/bin"
WASM_BINDGEN="$SCRATCH/bin/wasm-bindgen" scripts/build-web-gallery.sh "$SCRATCH/site"
ls -l "$SCRATCH/site"
grep -c '__WASM_SIZE__\|__VERSION__' "$SCRATCH/site/index.html"
```

Expected: `wasm-bindgen 0.2.125`; the site holds `index.html`, `gallery.js`, `gallery_bg.wasm`; the grep prints `0`. Record the size of `gallery_bg.wasm` and the gzip size (`gzip -c "$SCRATCH/site/gallery_bg.wasm" | wc -c`) for the PR description.

- [ ] **Step 6: Check the page in Chrome**

Serve with `python3 -m http.server 8765 --directory "$SCRATCH/site"` (in the background) and open `http://localhost:8765/` in a new Chrome tab with throttling (DevTools, Network, "Fast 4G") so the bar is visible.
Expected: the card shows `shadcn-egui 0.4.0 gallery` (0.5.0 after the release bump), the bar and `NN % · x.x / y.y MB` advance to 100 %, `Starting…` appears, then the card disappears and the gallery draws. No errors in the console.
Open `http://localhost:8765/?preset=b0&mode=dark`. Expected: the card is dark, the gallery is dark with preset `b0`.
Open `http://localhost:8765/?preset=zz`. Expected: the gallery starts with the default preset; the console shows `gallery: zz: …`.

- [ ] **Step 7: Check the page without a size**

Copy `web/index.html` unfilled over `"$SCRATCH/site/index.html"`, reload.
Expected: the status shows `x.x MB` without a percent, the gallery starts. Rebuild the site afterwards (Step 5).

- [ ] **Step 8: Check the error card**

Move `gallery_bg.wasm` aside, reload. Expected: `The download failed: ./gallery_bg.wasm: HTTP 404` and a `Try again` button. Move it back, click `Try again`, the gallery starts. Do the same with `gallery.js`: expected `The download failed: …` naming the failed module import. Stop the server.

- [ ] **Step 9: Commit**

```bash
git add web/index.html scripts/install-wasm-bindgen.sh scripts/build-web-gallery.sh
git commit -m "web: loading page with progress bar and the web gallery build scripts"
```

---

### Task 3: Web gallery build in CI

**Files:**
- Modify: `.github/workflows/ci-local.yml` (new job after `clippy-all-features`)

**Interfaces:**
- Consumes: both scripts from Task 2.
- Produces: the artifact `web-gallery` on every CI run.

- [ ] **Step 1: Add the job**

Insert after the `clippy-all-features` job:

```yaml
  # The browser gallery (scripts/build-web-gallery.sh). The workspace
  # clippy runs build for the host, where the wasm entry point of the
  # gallery example is compiled out, so it is linted here. The site is
  # uploaded so a reviewer can serve a pull request's gallery.
  web-gallery:
    name: Web gallery build
    runs-on: ubuntu-latest
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v7
        with:
          ref: ${{ inputs.ref }}

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
          components: clippy

      - uses: actions/cache@v6
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-wasm-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-wasm-

      - name: Install the wasm-bindgen CLI
        run: |
          scripts/install-wasm-bindgen.sh "$RUNNER_TEMP/bin"
          echo "$RUNNER_TEMP/bin" >> "$GITHUB_PATH"

      - run: cargo clippy --locked -p shadcn-egui --target wasm32-unknown-unknown --example gallery --features fonts-all,icons-all -- -D warnings

      - run: scripts/build-web-gallery.sh site

      - uses: actions/upload-artifact@v7
        with:
          name: web-gallery
          path: site
          retention-days: 7
```

Before committing, check the major version of `actions/upload-artifact` other workflows in `.github/workflows/` use (`grep -rn upload-artifact .github/workflows`) and use the same one; if none uses it, take the newest release (`gh release view -R actions/upload-artifact --json tagName`).

- [ ] **Step 2: Check `ci-passed` covers the job**

Read `.github/workflows/ci.yml`: the `local` job calls `ci-local.yml` and `ci-passed` needs `local`, so the new job is covered without a change. Confirm by reading, change nothing.

- [ ] **Step 3: Lint the workflow**

Run: `actionlint .github/workflows/ci-local.yml` if `actionlint` is on PATH; otherwise `python3 -c 'import sys,yaml; yaml.safe_load(open(sys.argv[1]))' .github/workflows/ci-local.yml`.
Expected: no output.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci-local.yml
git commit -m "ci: build and lint the web gallery"
```

The job proves itself on the PR's CI run (Task 5).

---

### Task 4: Pages deploy workflow

**Files:**
- Create: `.github/workflows/pages.yml`

**Interfaces:**
- Consumes: both scripts from Task 2; the workflow name `Publish release (automatic)` from `.github/workflows/release-publish.yml`.

- [ ] **Step 1: Write the workflow**

```yaml
name: Web gallery on GitHub Pages

# Builds the browser gallery from the newest release and publishes it on
# GitHub Pages. The release creates its tag and GitHub release with
# GITHUB_TOKEN, and events made with that token start no workflow, so a
# push-tags or release trigger would never fire; workflow_run does.
on:
  workflow_run:
    workflows: ["Publish release (automatic)"]
    types: [completed]
  workflow_dispatch:
    inputs:
      ref:
        description: Tag or branch to build; empty takes the newest release.
        type: string
        required: false
        default: ''

concurrency:
  group: pages
  cancel-in-progress: false

permissions:
  contents: read

jobs:
  build:
    name: Build the web gallery
    runs-on: ubuntu-latest
    timeout-minutes: 45
    outputs:
      deploy: ${{ steps.ref.outputs.deploy }}
    steps:
      # Publish release runs on every push to main that touches CHANGES.md,
      # most of which are ordinary merges. Its head_sha cannot identify a
      # release: the tag points to the commit built on the release branch
      # (v0.4.0 is 4ad8f37), not to the merge commit on main. A release this
      # run published is newer than the run.
      - name: Resolve the ref to build
        id: ref
        env:
          GH_TOKEN: ${{ github.token }}
          EVENT: ${{ github.event_name }}
          INPUT_REF: ${{ inputs.ref }}
          CONCLUSION: ${{ github.event.workflow_run.conclusion }}
          RUN_STARTED: ${{ github.event.workflow_run.run_started_at }}
        run: |
          latest=$(gh release view --repo "$GITHUB_REPOSITORY" --json tagName,publishedAt \
              --jq '.tagName + " " + .publishedAt')
          tag=${latest% *}
          published=${latest#* }
          if [ "$EVENT" = workflow_dispatch ]; then
            ref=${INPUT_REF:-$tag}
          elif [ "$CONCLUSION" != success ]; then
            echo "Publish release ended with $CONCLUSION; nothing to deploy." >> "$GITHUB_STEP_SUMMARY"
            echo deploy=false >> "$GITHUB_OUTPUT"
            exit 0
          elif [[ "$published" < "$RUN_STARTED" ]]; then
            echo "The newest release $tag ($published) predates this run ($RUN_STARTED): not a release, nothing to deploy." >> "$GITHUB_STEP_SUMMARY"
            echo deploy=false >> "$GITHUB_OUTPUT"
            exit 0
          else
            ref=$tag
          fi
          echo "Building the web gallery from $ref." >> "$GITHUB_STEP_SUMMARY"
          echo "ref=$ref" >> "$GITHUB_OUTPUT"
          echo deploy=true >> "$GITHUB_OUTPUT"

      - if: steps.ref.outputs.deploy == 'true'
        uses: actions/checkout@v7
        with:
          ref: ${{ steps.ref.outputs.ref }}

      - if: steps.ref.outputs.deploy == 'true'
        name: Check the ref has the web gallery
        env:
          REF: ${{ steps.ref.outputs.ref }}
        run: |
          if [ ! -x scripts/build-web-gallery.sh ]; then
            echo "::error::$REF has no scripts/build-web-gallery.sh; the web gallery exists from 0.5.0 on."
            exit 1
          fi

      - if: steps.ref.outputs.deploy == 'true'
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - if: steps.ref.outputs.deploy == 'true'
        uses: actions/cache@v6
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-wasm-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-wasm-

      - if: steps.ref.outputs.deploy == 'true'
        name: Build
        run: |
          scripts/install-wasm-bindgen.sh "$RUNNER_TEMP/bin"
          WASM_BINDGEN="$RUNNER_TEMP/bin/wasm-bindgen" scripts/build-web-gallery.sh site

      - if: steps.ref.outputs.deploy == 'true'
        uses: actions/upload-pages-artifact@v5
        with:
          path: site

  deploy:
    name: Deploy to GitHub Pages
    needs: build
    if: needs.build.outputs.deploy == 'true'
    runs-on: ubuntu-latest
    timeout-minutes: 10
    permissions:
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v5
```

The timestamps compare as strings: both are ISO 8601 UTC (`2026-10-15T12:00:00Z`) from GitHub.

- [ ] **Step 2: Lint the workflow**

Run: `actionlint .github/workflows/pages.yml` if on PATH; otherwise the `yaml.safe_load` check from Task 3, step 3.
Expected: no output.

- [ ] **Step 3: Dry-run the resolve step locally**

Extract the `run:` script of "Resolve the ref to build" into a scratch file and run it with `GITHUB_REPOSITORY=oetiker/shadcn-egui`, `GITHUB_OUTPUT` and `GITHUB_STEP_SUMMARY` pointing at scratch files:
- `EVENT=workflow_run CONCLUSION=success RUN_STARTED=2026-10-15T00:00:00Z`: expected `deploy=false`, summary says `v0.4.0 … predates this run`.
- `EVENT=workflow_run CONCLUSION=success RUN_STARTED=2026-10-01T00:00:00Z`: expected `deploy=true`, `ref=v0.4.0`.
- `EVENT=workflow_run CONCLUSION=failure`: expected `deploy=false`.
- `EVENT=workflow_dispatch INPUT_REF=`: expected `ref=v0.4.0`; with `INPUT_REF=main`: `ref=main`.
Then check the guard: `git stash` is not needed; run `git show v0.4.0:scripts/build-web-gallery.sh` and expect "does not exist", which is the condition the guard step reports.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/pages.yml
git commit -m "pages: deploy the web gallery after each release"
```

---

### Task 5: Documentation, gates and the PR

**Files:**
- Modify: `README.md` (after the screenshot line)
- Modify: `CHANGES.md` (`### New` under `## [Unreleased]`)
- Modify: `RELEASING.md` (end of `## Cutting a release`)
- Modify: `CLAUDE.md` (`## Layout`, `## Build & verify`)

- [ ] **Step 1: README**

After the line `![shadcn-egui component gallery](…)`, add:

```markdown
[Try the gallery in your browser](https://oetiker.github.io/shadcn-egui/):
every component under the preset pickers, with all fonts and icon sets. The
first load downloads about <gzip size from Task 2, step 5> MB.
```

- [ ] **Step 2: CHANGES**

Add as the first item under `### New`:

```markdown
- The component gallery runs in the browser at
  https://oetiker.github.io/shadcn-egui/ and is updated with each release.
  A link such as `?preset=b0&mode=dark` opens it with that preset and mode.
```

- [ ] **Step 3: RELEASING**

At the end of `## Cutting a release`, add:

```markdown
After Publish release finishes, the workflow "Web gallery on GitHub Pages"
builds the gallery from the new release and deploys it. To deploy again, or
to deploy a branch, run that workflow by hand; its `ref` input takes a tag
or a branch and defaults to the newest release.
```

- [ ] **Step 4: CLAUDE.md**

Under `## Layout`, add after the `tools/` item:

```markdown
- `web/index.html` — the browser gallery's loading page;
  `scripts/build-web-gallery.sh <out-dir>` builds the site,
  `scripts/install-wasm-bindgen.sh <dir>` fetches the matching
  `wasm-bindgen` CLI. `.github/workflows/pages.yml` deploys it after each
  release.
```

Keep the existing em dash style of that list, which `CLAUDE.md` already uses for every item. Under `## Build & verify`, in the gates block after the second clippy line, add:

```bash
cargo clippy -p shadcn-egui --target wasm32-unknown-unknown --example gallery --features fonts-all,icons-all --jobs 4 -- -D warnings
```

- [ ] **Step 5: Run every gate**

Run each command of the gates block in `CLAUDE.md`, including the new wasm clippy line, with `timeout: 600000`. The stylegen drift check needs the shadcn-svelte and shadcn-ui checkouts; if they are not available, say so in the PR instead of skipping silently.
Expected: all pass, warning-clean. Record the test count delta: the 4 new `initial_state` tests run only with `--all-features`.

- [ ] **Step 6: Check for private paths**

Run: `git diff main --stat` and `git diff main | grep -n '/home/\|/scratch/'`.
Expected: no matches.

- [ ] **Step 7: Commit, push, open the PR**

```bash
git add README.md CHANGES.md RELEASING.md CLAUDE.md
git commit -m "docs: the web gallery in README, CHANGES, RELEASING and CLAUDE.md"
git push -u origin feat/web-gallery
gh pr create --title "Web gallery on GitHub Pages" --body "<summary, wasm and gzip sizes, manual check results, the one-time Pages setting below>"
```

The PR body names the one-time setting for the maintainer:

```
! gh api -X POST repos/oetiker/shadcn-egui/pages -f build_type=workflow
```

- [ ] **Step 8: Watch CI**

Expected: `Web gallery build` green, the `web-gallery` artifact present; download it and serve it once as in Task 2, step 6. After the merge, the maintainer runs "Web gallery on GitHub Pages" by hand with `ref: main` to see the first deploy before 0.5.0.
