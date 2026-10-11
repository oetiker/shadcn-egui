//! The registry copy is what projects vendor; it must equal `src/`.

use std::path::{Path, PathBuf};

fn files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).expect("dir") {
            let p = e.expect("entry").path();
            if p.is_dir() {
                stack.push(p)
            } else {
                out.push(p.strip_prefix(root).unwrap().to_path_buf())
            }
        }
    }
    out.sort();
    out
}

#[test]
fn registry_src_is_byte_identical() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (src, reg) = (
        root.join("src"),
        root.join("skills/egui-shadcn/registry/src"),
    );
    let (a, b) = (files(&src), files(&reg));
    assert_eq!(a, b, "file lists differ");
    assert!(a.len() > 10, "too few files: {a:?}");
    for f in a {
        assert!(
            std::fs::read(src.join(&f)).unwrap() == std::fs::read(reg.join(&f)).unwrap(),
            "{} differs",
            f.display()
        );
    }
}
