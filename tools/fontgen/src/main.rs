//! `fontgen <downloads-dir> <repo-root>`: see the crate docs.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [downloads, repo] = &args[..] else {
        eprintln!("usage: fontgen <downloads-dir> <repo-root>");
        return ExitCode::FAILURE;
    };
    match fontgen::generate(Path::new(downloads), Path::new(repo)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fontgen: {e}");
            ExitCode::FAILURE
        }
    }
}
