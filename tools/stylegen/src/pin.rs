//! Upstream commit pins: validation and the check against a checkout.

use std::path::Path;
use std::process::Command;

/// Twin of `parse_pin` in tools/fontgen/src/lib.rs (separate workspaces); keep both in step.
/// A pin is an abbreviated or full git commit id: 7 to 40 lowercase hex
/// characters, surrounding whitespace ignored.
pub fn parse_pin(text: &str) -> Result<String, String> {
    let pin = text.trim();
    if (7..=40).contains(&pin.len()) && pin.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        Ok(pin.to_owned())
    } else {
        Err(format!("{pin:?} is not 7 to 40 lowercase hex characters"))
    }
}

/// Fails unless the checkout at `up` is at the commit `pin` names.
pub fn check_pin(up: &Path, pin: &str) -> Result<(), String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(up)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git rev-parse HEAD in {} failed ({}): {}",
            up.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let head = String::from_utf8_lossy(&out.stdout);
    let head = head.trim();
    if head.starts_with(pin) {
        Ok(())
    } else {
        Err(format!(
            "upstream checkout {} is at {head}, PIN says {pin}",
            up.display()
        ))
    }
}
