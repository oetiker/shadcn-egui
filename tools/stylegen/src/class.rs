/// One Tailwind class split into variants and utility.
#[derive(Debug, PartialEq)]
pub struct Class {
    pub variants: Vec<String>,
    pub utility: String,
    pub important: bool,
}

/// Split at top-level colons; `[]` and `()` protect colons inside arbitrary
/// values and selectors.
pub fn split(class: &str) -> Class {
    let mut variants = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    for (i, ch) in class.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ':' if depth == 0 => {
                variants.push(class[start..i].to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }
    let raw = &class[start..];
    let (utility, important) = match (raw.strip_suffix('!'), raw.strip_prefix('!')) {
        (Some(u), _) | (_, Some(u)) => (u.to_owned(), true),
        _ => (raw.to_owned(), false),
    };
    Class {
        variants,
        utility,
        important,
    }
}
