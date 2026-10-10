/// Parse a shadcn `style-<s>.css` into `marker → classes`, following
/// upstream's `parseStyleCss` (`packages/parity/src/inject-style-classes.ts`).
///
/// A rule's subject is the last `cn-*` identifier of each comma-separated
/// selector part. Only the rule's own `@apply` lists count (nested rules are
/// walked separately), and an `@apply` may omit its `;` before `}`. When a
/// marker is defined by several rules, the later rule's classes are placed
/// *before* the earlier ones, as upstream does. Markers are returned in order
/// of first appearance.
pub fn parse_style_css(src: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    walk(&strip_comments(src), &mut out);
    out
}

fn walk(css: &str, out: &mut Vec<(String, Vec<String>)>) {
    let mut i = 0;
    while let Some(open_rel) = css[i..].find('{') {
        let open = i + open_rel;
        let selector = css[i..open].trim();
        let Some(close) = matching_brace(css, open) else { return };
        let body = &css[open + 1..close];
        if !selector.starts_with('@') || selector.starts_with("@apply") {
            let apply = immediate_apply(body);
            if !apply.is_empty() {
                for part in selector.split(',') {
                    if let Some(marker) = subject_marker(part.trim()) {
                        add(out, marker, &apply);
                    }
                }
            }
        }
        walk(body, out);
        i = close + 1;
    }
}

/// Later rules go in front of earlier ones.
fn add(out: &mut Vec<(String, Vec<String>)>, marker: String, apply: &[String]) {
    match out.iter_mut().find(|(m, _)| *m == marker) {
        Some((_, v)) => {
            v.splice(0..0, apply.iter().cloned());
        }
        None => out.push((marker, apply.to_vec())),
    }
}

fn matching_brace(css: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, ch) in css[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// The classes of every `@apply` that sits directly in `body`, not inside a
/// nested rule.
fn immediate_apply(body: &str) -> Vec<String> {
    let mut direct = String::new();
    let mut depth = 0usize;
    for ch in body.chars() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => direct.push(ch),
            _ => {}
        }
    }
    let mut classes = Vec::new();
    let mut rest = direct.as_str();
    while let Some(at) = rest.find("@apply") {
        let tail = &rest[at + "@apply".len()..];
        if !tail.starts_with(char::is_whitespace) {
            rest = tail;
            continue;
        }
        let end = tail.find(';').unwrap_or(tail.len());
        classes.extend(tail[..end].split_whitespace().map(str::to_owned));
        rest = &tail[end..];
    }
    classes
}

fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        rest = rest[i..].find("*/").map_or("", |j| &rest[i + j + 2..]);
    }
    out.push_str(rest);
    out
}

/// Last `cn-*` identifier that starts the selector or follows whitespace or
/// one of `. > + ~`.
fn subject_marker(selector: &str) -> Option<String> {
    let mut found = None;
    let mut from = 0;
    while let Some(rel) = selector[from..].find("cn-") {
        let i = from + rel;
        let tail = &selector[i..];
        let len = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(tail.len());
        let at_boundary = selector[..i]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || ".>+~".contains(c));
        if at_boundary {
            found = Some(tail[..len].to_owned());
        }
        from = i + len;
    }
    found
}
