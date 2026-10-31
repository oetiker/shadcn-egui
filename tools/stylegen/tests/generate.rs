use std::path::{Path, PathBuf};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("mkdir");
    for e in std::fs::read_dir(from)
        .expect("read_dir")
        .map(Result::unwrap)
    {
        let dest = to.join(e.file_name());
        if e.file_type().expect("file type").is_dir() {
            copy_dir(&e.path(), &dest);
        } else {
            std::fs::copy(e.path(), dest).expect("copy");
        }
    }
}

/// A scratch copy of the tool's inputs and of the crate files generate writes.
fn scratch(name: &str) -> (PathBuf, PathBuf) {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&base);
    let (tool, root) = (base.join("tool"), base.join("root"));
    copy_dir(
        &Path::new(ROOT).join("tests/fixtures"),
        &tool.join("tests/fixtures"),
    );
    for f in ["markers.txt", "EXPORT"] {
        std::fs::copy(Path::new(ROOT).join(f), tool.join(f)).expect("copy");
    }
    let krate = Path::new(ROOT).join("../../crates/shadcn-egui/src");
    for dir in ["style", "palette", "preset"] {
        copy_dir(
            &krate.join(dir),
            &root.join("crates/shadcn-egui/src").join(dir),
        );
    }
    (tool, root)
}

/// Every file under `dir`, with its bytes, sorted by path.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir)
        .expect("read_dir")
        .map(Result::unwrap)
    {
        if e.file_type().expect("file type").is_dir() {
            out.extend(snapshot(&e.path()));
        } else {
            out.push((e.path(), std::fs::read(e.path()).expect("read")));
        }
    }
    out.sort();
    out
}

/// A failed run must leave the crate as it was: a half-written
/// `generated.rs` without the `Recipes` fields does not compile.
#[test]
fn failed_generate_leaves_the_crate_untouched() {
    let (tool, root) = scratch("failed_generate");
    // One unsupported class in every exported style, as a new marker
    // brings: with no style left, `Recipes` would lose every field.
    for style in std::fs::read_to_string(tool.join("EXPORT"))
        .expect("EXPORT")
        .split_whitespace()
    {
        let css = tool.join(format!("tests/fixtures/style-{style}.css"));
        let text = std::fs::read_to_string(&css).expect("read");
        let broken = text.replacen(
            ".cn-progress {\n    @apply ",
            ".cn-progress {\n    @apply border-r-input ",
            1,
        );
        assert_ne!(text, broken, "{style}: fixture changed; pick another rule");
        std::fs::write(&css, broken).expect("write");
    }
    let before = snapshot(&root);
    let pin = std::fs::read_to_string(Path::new(ROOT).join("PIN")).expect("PIN");
    let pin_react = std::fs::read_to_string(Path::new(ROOT).join("PIN_REACT")).expect("PIN");
    let errors = stylegen::generate::generate(&tool, &root, pin.trim(), pin_react.trim())
        .expect_err("nova has an unsupported class");
    assert!(
        errors.iter().any(|e| e.starts_with("nova is in EXPORT")),
        "{errors:?}"
    );
    assert!(before == snapshot(&root), "generate wrote crate files");
}

/// The clean run writes the same crate files the repository holds.
#[test]
fn clean_generate_writes_the_committed_files() {
    let (tool, root) = scratch("clean_generate");
    let before = snapshot(&root);
    let pin = std::fs::read_to_string(Path::new(ROOT).join("PIN")).expect("PIN");
    let pin_react = std::fs::read_to_string(Path::new(ROOT).join("PIN_REACT")).expect("PIN");
    stylegen::generate::generate(&tool, &root, pin.trim(), pin_react.trim()).expect("generate");
    assert!(
        before == snapshot(&root),
        "output differs from the repository"
    );
}
