//! Class strings of the shadcn-svelte component sources
//! (`docs/src/lib/registry/ui/<dir>/*.svelte`), and the string of each part.

use crate::markers::{is_cli_marker, MarkerLine};
use std::path::Path;

/// A part a component file draws in several `{#if}` branches:
/// `(file under ui/, marker, occurrence the crate draws (0-based), why)`.
pub const BRANCHES: &[(&str, &str, usize, &str)] = &[(
    "input/input.svelte",
    "cn-input",
    1,
    "input.rs draws a text field (egui TextEdit), never type=\"file\": the {:else} branch",
)];

/// Every class string in a component source, in source order: the string
/// arguments of each `cn(…)` joined with a space, the `base` string and each
/// string in `variants` of a `tv({…})`, and each static `class="…"`, also
/// on a nested component. HTML comments and `//` comments are skipped, and
/// so are identifier arguments (`className`, `arrowClasses`) and calls
/// (`buttonVariants(..)`). Syntax whose value depends on runtime state
/// (`open && "y"`, `a ? "p" : "q"`, `{"z": on}`, `${..}` in a template
/// literal, `{..}` in `class="…"`, a member call `a.b(..)`) is an error: it
/// has no single string. So are forms the extractor does not read (a `/* */`
/// comment in `cn(`, `tv({ slots: .. })`, `class={"…"}`, `class='…'`):
/// skipping them would let a marker resolve from another file.
pub fn class_strings(src: &str) -> Result<Vec<String>, String> {
    let src = mask_js_comments(&strip_html_comments(src));
    unread_class_attribute(&src)?;
    let mut found: Vec<(usize, String)> = Vec::new();
    for (at, args) in calls(&src, "cn")? {
        found.push((at, cn_strings(args)?.join(" ")));
    }
    for (at, args) in calls(&src, "tv")? {
        for s in tv_strings(args)? {
            found.push((at, s));
        }
    }
    for at in class_attributes(&src) {
        if !src[at..].starts_with('"') {
            continue;
        }
        let start = at + 1;
        let Some(len) = src[start..].find('"') else {
            break;
        };
        let value = &src[start..start + len];
        if value.contains('{') {
            return Err(format!("dynamic class attribute: class=\"{value}\""));
        }
        found.push((start, value.to_owned()));
    }
    // Stable: the strings of one `tv` keep their order.
    found.sort_by_key(|(at, _)| *at);
    Ok(found
        .into_iter()
        .map(|(_, s)| normalise(&s))
        .filter(|s| !s.is_empty())
        .collect())
}

/// Whitespace collapsed to single spaces.
pub fn normalise(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `s` without the markers only the shadcn CLI rewrites.
pub fn strip_cli_markers(s: &str) -> String {
    s.split_whitespace()
        .filter(|t| !(t.starts_with("cn-") && is_cli_marker(t)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The class string of every part in `lines`, read from
/// `<ui>/<dir>/*.svelte`: `(part, classes)` in `markers.txt` order. A
/// `cn-*` part's classes still hold its marker. Every problem is named in
/// the error: a part no string carries, a string with two shipped markers,
/// several differing strings for one part, several strings in one file
/// without a [`BRANCHES`] entry.
pub fn part_strings(ui: &Path, lines: &[MarkerLine]) -> Result<Vec<(String, Vec<String>)>, String> {
    let shipped: Vec<&str> = lines
        .iter()
        .flat_map(|l| l.markers.iter().map(String::as_str))
        .filter(|m| !m.starts_with("slot:"))
        .collect();
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for line in lines {
        if line.markers.is_empty() {
            continue;
        }
        let files =
            svelte_sources(&ui.join(&line.dir)).map_err(|e| format!("ui/{}: {e}", line.dir))?;
        for part in &line.markers {
            match part_string(&line.dir, &files, part, &shipped) {
                Ok(s) => out.push((
                    part.clone(),
                    s.split_whitespace().map(str::to_owned).collect(),
                )),
                Err(e) => problems.push(e),
            }
        }
    }
    if problems.is_empty() {
        Ok(out)
    } else {
        Err(problems.join("\n"))
    }
}

/// `(file name, class strings without CLI markers)` of every `*.svelte`
/// in `dir`, by name.
fn svelte_sources(dir: &Path) -> std::io::Result<Vec<(String, Vec<String>)>> {
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".svelte"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|n| {
            let src = std::fs::read_to_string(dir.join(&n))?;
            let strings = class_strings(&src)
                .map_err(|e| std::io::Error::other(format!("{n}: {e}")))?
                .iter()
                .map(|s| strip_cli_markers(s))
                .collect();
            Ok((n, strings))
        })
        .collect()
}

fn part_string(
    dir: &str,
    files: &[(String, Vec<String>)],
    part: &str,
    shipped: &[&str],
) -> Result<String, String> {
    if let Some(slot) = part.strip_prefix("slot:") {
        let file = format!("{slot}.svelte");
        let strings = files
            .iter()
            .find(|(n, _)| *n == file)
            .map(|(_, s)| s)
            .ok_or_else(|| format!("{part}: no ui/{dir}/{file}"))?;
        let plain: Vec<&String> = strings
            .iter()
            .filter(|s| !s.split_whitespace().any(|t| t.starts_with("cn-")))
            .collect();
        return match plain.as_slice() {
            [one] => Ok((*one).clone()),
            _ => Err(format!(
                "{part}: ui/{dir}/{file} has {} class strings without a marker, expected one",
                plain.len()
            )),
        };
    }
    let mut chosen: Vec<String> = Vec::new();
    for (name, strings) in files {
        let hits: Vec<&String> = strings
            .iter()
            .filter(|s| s.split_whitespace().any(|t| t == part))
            .collect();
        let pick: &String = match hits.len() {
            0 => continue,
            1 => hits[0],
            n => {
                let path = format!("{dir}/{name}");
                let Some((_, _, k, _)) = BRANCHES
                    .iter()
                    .find(|(f, m, _, _)| *f == path && *m == part)
                else {
                    return Err(format!(
                        "{part}: ui/{path} has {n} class strings for it; name the one the crate draws in extract::BRANCHES"
                    ));
                };
                hits.get(*k).copied().ok_or_else(|| {
                    format!("{part}: BRANCHES names occurrence {k}, ui/{path} has {n}")
                })?
            }
        };
        let others: Vec<&str> = pick
            .split_whitespace()
            .filter(|t| *t != part && shipped.contains(t))
            .collect();
        if !others.is_empty() {
            return Err(format!(
                "{part}: its string in ui/{dir}/{name} also holds {others:?}"
            ));
        }
        if !chosen.contains(pick) {
            chosen.push(pick.clone());
        }
    }
    match chosen.len() {
        0 => Err(format!("{part}: no class string in ui/{dir}")),
        1 => Ok(chosen.remove(0)),
        _ => Err(format!(
            "{part}: differing class strings in ui/{dir}: {chosen:?}"
        )),
    }
}

fn strip_html_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(i) = rest.find("<!--") {
        out.push_str(&rest[..i]);
        rest = rest[i..].find("-->").map_or("", |j| &rest[i + j + 3..]);
    }
    out.push_str(rest);
    out
}

/// `(position, argument text)` of every call `name(…)` where `name` is not
/// the tail of a longer identifier or a method. A call that never closes
/// (an unclosed string or bracket in it) is an error: skipping it would
/// drop its classes silently.
fn calls<'a>(src: &'a str, name: &str) -> Result<Vec<(usize, &'a str)>, String> {
    let pat = format!("{name}(");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = src[from..].find(&pat) {
        let at = from + i;
        from = at + pat.len();
        let joined = src[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '$');
        if joined {
            continue;
        }
        let end = closing(src, from - 1).ok_or_else(|| {
            format!(
                "{name}() call that does not close (an unclosed string or bracket): {}",
                src[at..].lines().next().unwrap_or("").trim()
            )
        })?;
        out.push((at, &src[from..end]));
        from = end + 1;
    }
    Ok(out)
}

/// Index of the bracket closing the one at `open`, skipping strings.
/// Comments are masked already ([`mask_js_comments`]); a `//` it leaves
/// (after `:`, a URL) is text.
fn closing(src: &str, open: usize) -> Option<usize> {
    let b = src.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' | b'`' => i = skip_string(b, i)?,
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Index of the quote closing the string that starts at `i`.
fn skip_string(b: &[u8], i: usize) -> Option<usize> {
    let q = b[i];
    let mut j = i + 1;
    while j < b.len() {
        if b[j] == b'\\' {
            j += 2;
            continue;
        }
        if b[j] == q {
            return Some(j);
        }
        j += 1;
    }
    None
}

/// The string arguments of a `cn(…)` call. Identifiers, member chains and
/// plain calls are skipped; a member call and any other argument is an
/// error.
fn cn_strings(args: &str) -> Result<Vec<String>, String> {
    let b = args.as_bytes();
    let (mut out, mut i) = (Vec::new(), 0usize);
    let bad = |at: usize| {
        Err(format!(
            "cn() argument the extractor cannot evaluate: {}",
            args[at..].lines().next().unwrap_or("").trim()
        ))
    };
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() || c == b',' {
            i += 1;
            continue;
        }
        let start = i;
        if matches!(c, b'"' | b'\'' | b'`') {
            let Some(end) = skip_string(b, i) else {
                return bad(start);
            };
            check_literal(&args[i + 1..end], c)?;
            out.push(args[i + 1..end].to_owned());
            i = end + 1;
        } else if c.is_ascii_alphabetic() || matches!(c, b'_' | b'$') {
            while i < b.len()
                && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'_' | b'$' | b'.'))
            {
                i += 1;
            }
            if b.get(i) == Some(&b'(') {
                if args[start..i].contains('.') {
                    return Err(format!(
                        "cn() member call the extractor cannot evaluate: {}",
                        args[start..].lines().next().unwrap_or("").trim()
                    ));
                }
                let Some(end) = closing(args, i) else {
                    return bad(start);
                };
                i = end + 1;
            }
        } else {
            return bad(start);
        }
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < b.len() && b[i] != b',' {
            return bad(start);
        }
    }
    Ok(out)
}

/// `s` with every string literal's body removed (quotes kept), so a
/// search finds only code.
fn strings_free(s: &str) -> String {
    let b = s.as_bytes();
    let (mut out, mut i) = (String::with_capacity(s.len()), 0usize);
    while i < b.len() {
        if matches!(b[i], b'"' | b'\'' | b'`') {
            let end = skip_string(b, i).unwrap_or(b.len() - 1);
            out.push_str("\"\"");
            i = end + 1;
        } else {
            let c = s[i..].chars().next().expect("char boundary");
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

fn check_literal(body: &str, quote: u8) -> Result<(), String> {
    if quote == b'`' && body.contains("${") {
        return Err(format!("template literal with interpolation: `{body}`"));
    }
    Ok(())
}

/// `src` with `//` comments overwritten by spaces (same length), so
/// positions stay valid. A `//` inside a quoted string, or right after `:`
/// (a URL), is not a comment. A `"` or `'` string ends with its line; a
/// template literal (`` ` ``) runs on across lines. An apostrophe right
/// after a letter or digit (`it's` in markup text) opens no string. A
/// template literal opens only in JavaScript: inside `<script>` or a
/// `{..}` expression in the markup. A backtick in markup text is text.
fn mask_js_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut quote = 0u8;
    // Inside `<script>`, and the `{..}` nesting of the markup outside it.
    let (mut script, mut depth) = (false, 0usize);
    for line in src.split_inclusive('\n') {
        let b = line.as_bytes();
        if quote != b'`' {
            quote = 0;
        }
        let mut cut = None;
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            if quote != 0 {
                if c == b'\\' {
                    i += 1;
                } else if c == quote {
                    quote = 0;
                }
            } else if c == b'"'
                || (c == b'`' && (script || depth > 0))
                || (c == b'\'' && (i == 0 || !b[i - 1].is_ascii_alphanumeric()))
            {
                quote = c;
            } else if c == b'/' && b.get(i + 1) == Some(&b'/') && (i == 0 || b[i - 1] != b':') {
                cut = Some(i);
                break;
            } else if script {
                script = !b[i..].starts_with(b"</script");
            } else if c == b'{' {
                depth += 1;
            } else if c == b'}' {
                depth = depth.saturating_sub(1);
            } else if depth == 0 && b[i..].starts_with(b"<script") {
                script = true;
            }
            i += 1;
        }
        match cut {
            Some(at) => {
                out.push_str(&line[..at]);
                out.extend(
                    line[at..]
                        .chars()
                        .map(|c| if c == '\n' { '\n' } else { ' ' }),
                );
            }
            None => out.push_str(line),
        }
    }
    out
}

/// String literals of `args` (comments masked already), at depth 0 only or
/// at any depth. A string followed by `:` right after `{` or `,` is an
/// object key (`"icon-xs":`), not a value, and is left out. A `?` (a
/// conditional, `a ? "x" : "y"`, or `?.`, `??`) is an error: the value
/// depends on runtime state.
fn strings(args: &str, any_depth: bool) -> Result<Vec<String>, String> {
    let b = args.as_bytes();
    let (mut out, mut depth, mut i) = (Vec::new(), 0i32, 0usize);
    // The last byte outside strings and whitespace.
    let mut prev = 0u8;
    let conditional = |at: usize| {
        Err(format!(
            "conditional value the extractor cannot evaluate: {}",
            args[at..].lines().next().unwrap_or("").trim()
        ))
    };
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' | b'`' => {
                let end = skip_string(b, i).ok_or_else(|| unclosed(args, i))?;
                let colon = args[end + 1..].trim_start().starts_with(':');
                let key = colon && matches!(prev, b'{' | b',');
                if colon && !key {
                    return conditional(i);
                }
                if (any_depth || depth == 0) && !key {
                    check_literal(&args[i + 1..end], b[i])?;
                    out.push(args[i + 1..end].to_owned());
                }
                prev = b[i];
                i = end;
            }
            b'?' => return conditional(i),
            c => {
                match c {
                    b'(' | b'{' | b'[' => depth += 1,
                    b')' | b'}' | b']' => depth -= 1,
                    _ => {}
                }
                if !c.is_ascii_whitespace() {
                    prev = c;
                }
            }
        }
        i += 1;
    }
    Ok(out)
}

/// The error for a string that `args` opens at `at` and never closes: it
/// would hide every key and value after it.
fn unclosed(args: &str, at: usize) -> String {
    format!(
        "unclosed string the extractor cannot read: {}",
        args[at..].lines().next().unwrap_or("").trim()
    )
}

/// The keys `tv({…})` reads. `defaultVariants` names variants and holds no
/// classes, so it contributes nothing.
const TV_KEYS: &[&str] = &["base", "variants", "defaultVariants"];

/// The `base` string and every string value in `variants` of a `tv({…})`
/// call. Any other top-level key (`slots`, `compoundVariants`) is an error:
/// the extractor does not read it, and dropping it would lose its classes.
fn tv_strings(args: &str) -> Result<Vec<String>, String> {
    if let Some(k) = object_keys(args)?
        .into_iter()
        .find(|k| !TV_KEYS.contains(&k.as_str()))
    {
        return Err(format!(
            "tv() key the extractor does not read: {k} in {}",
            args.trim().lines().next().unwrap_or("")
        ));
    }
    let mut out = Vec::new();
    if let Some(i) = top_key(args, "base")? {
        out.extend(strings(&args[i..], false)?.into_iter().take(1));
    }
    if let Some(i) = top_key(args, "variants")? {
        if let Some(open) = args[i..].find('{').map(|o| i + o) {
            if let Some(close) = closing(args, open) {
                out.extend(strings(&args[open..=close], true)?);
            }
        }
    }
    Ok(out)
}

/// Every key of the object that `args` (a call's argument text) opens: a
/// bare name or a string at bracket depth 1, right after `{` or `,` and
/// followed by `:`. A key whose name or value the extractor cannot see is
/// an error: a shorthand (`{ base }`), a spread (`...x`), a computed key
/// (`[k]:`), a method or a string not followed by `:`.
fn object_keys(args: &str) -> Result<Vec<String>, String> {
    let b = args.as_bytes();
    let (mut out, mut depth, mut i) = (Vec::new(), 0i32, 0usize);
    let mut prev = 0u8;
    let colon_after = |end: usize| args[end..].trim_start().starts_with(':');
    let unreadable = |at: usize| {
        Err(format!(
            "tv() key the extractor cannot evaluate: {}",
            args[at..].lines().next().unwrap_or("").trim()
        ))
    };
    while i < b.len() {
        let c = b[i];
        let at_key = depth == 1 && matches!(prev, b'{' | b',');
        if matches!(c, b'"' | b'\'' | b'`') {
            let end = skip_string(b, i).ok_or_else(|| unclosed(args, i))?;
            if at_key {
                if !colon_after(end + 1) {
                    return unreadable(i);
                }
                out.push(args[i + 1..end].to_owned());
            }
            prev = c;
            i = end + 1;
            continue;
        }
        if at_key && (c.is_ascii_alphabetic() || matches!(c, b'_' | b'$')) {
            let len = args[i..]
                .bytes()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'$'))
                .count();
            if !colon_after(i + len) {
                return unreadable(i);
            }
            out.push(args[i..i + len].to_owned());
            prev = b'a';
            i += len;
            continue;
        }
        if at_key && matches!(c, b'.' | b'[') {
            return unreadable(i);
        }
        match c {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            _ => {}
        }
        if !c.is_ascii_whitespace() {
            prev = c;
        }
        i += 1;
    }
    Ok(out)
}

/// The index after `key:` where `key` is a whole bare key of the object
/// that `args` (a call's argument text) opens: at bracket depth 1, outside
/// strings, not the tail of a longer name (`database:`). An unclosed
/// string is an error: it would hide every later key.
fn top_key(args: &str, key: &str) -> Result<Option<usize>, String> {
    let b = args.as_bytes();
    let (mut depth, mut i) = (0i32, 0usize);
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' | b'`' => i = skip_string(b, i).ok_or_else(|| unclosed(args, i))?,
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            _ if depth == 1
                && b[i..].starts_with(key.as_bytes())
                && (i == 0
                    || !(b[i - 1].is_ascii_alphanumeric() || matches!(b[i - 1], b'_' | b'$'))) =>
            {
                let rest = &args[i + key.len()..];
                let after = rest.trim_start();
                if let Some(value) = after.strip_prefix(':') {
                    return Ok(Some(args.len() - value.len()));
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok(None)
}

/// Where each `class=` attribute starts: `class` as a whole attribute name,
/// not the tail of a longer one (`subclass=`, `data-class=`).
fn class_attributes(src: &str) -> impl Iterator<Item = usize> + '_ {
    src.match_indices("class=").filter_map(move |(i, _)| {
        let joined = src[..i]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | ':' | '$'));
        (!joined).then_some(i + "class=".len())
    })
}

/// An error for a `class` attribute the extractor does not read: a string
/// in braces (`class={"…"}`), in single quotes (`class='…'`), a Svelte 5
/// array or object (`class={[..]}`, `class={{..}}`) or a conditional
/// (`class={c ? "a" : "b"}`), and so is a `?` anywhere outside strings,
/// also in a call's arguments (`class={f(a?.b)}`). An identifier or a call
/// without one (`class={cn(..)}`) is fine: the call is read where it
/// stands. A call nested in its arguments is skipped as [`cn_strings`]
/// skips it, so a `?` there decides nothing about the part's string
/// (`cn(buttonVariants({ variant: active ? "outline" : "ghost" }), "cn-x")`:
/// the composed recipes are the runtime's).
fn unread_class_attribute(src: &str) -> Result<(), String> {
    for at in class_attributes(src) {
        let rest = &src[at..];
        let braced = rest.strip_prefix('{').is_some_and(|r| {
            let r = r.trim_start();
            if r.starts_with(['"', '\'', '`', '[', '{']) {
                return true;
            }
            // A `?` outside strings, also inside a call: a conditional.
            closing(rest, 0).is_some_and(|end| {
                strings_free(&without_nested_calls(&rest[1..end])).contains('?')
            })
        });
        if braced || rest.starts_with('\'') {
            return Err(format!(
                "class attribute form the extractor cannot read: class={}",
                rest.lines().next().unwrap_or("")
            ));
        }
    }
    Ok(())
}

/// `expr` with the argument text of every call nested inside the arguments
/// of its outermost call blanked out (`cn(f(a ? b), "x")` keeps `cn(`,
/// `f()` and `"x"`). Strings are skipped while looking for calls.
fn without_nested_calls(expr: &str) -> String {
    let b = expr.as_bytes();
    let Some(open) = expr.find('(') else {
        return expr.to_owned();
    };
    let mut out = String::with_capacity(expr.len());
    out.push_str(&expr[..=open]);
    let mut i = open + 1;
    while i < b.len() {
        let c = b[i];
        if matches!(c, b'"' | b'\'' | b'`') {
            let end = skip_string(b, i).unwrap_or(b.len() - 1);
            out.push_str(&expr[i..=end]);
            i = end + 1;
        } else if c == b'(' && i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_') {
            let end = closing(expr, i).unwrap_or(b.len() - 1);
            out.push_str("()");
            i = end + 1;
        } else {
            let ch = expr[i..].chars().next().expect("char boundary");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}
