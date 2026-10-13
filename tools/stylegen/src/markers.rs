/// One line of `markers.txt`.
#[derive(Debug, PartialEq)]
pub struct MarkerLine {
    pub file: String,
    pub dir: String,
    pub markers: Vec<String>,
}

pub fn parse_markers(src: &str) -> Vec<MarkerLine> {
    src.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (head, markers) = l.split_once(':').expect("`<file> <dir>: <markers>`");
            let mut h = head.split_whitespace();
            let file = h.next().expect("file").to_owned();
            let dir = h.next().expect("dir").to_owned();
            MarkerLine {
                file,
                dir,
                markers: markers.split_whitespace().map(str::to_owned).collect(),
            }
        })
        .collect()
}

/// Markers only the shadcn CLI rewrites (fonts, menu targets, RTL and
/// logical-side helpers). The registry keeps some as text and drops the
/// classes of `*-logical`; the crate models none of them.
pub fn is_cli_marker(m: &str) -> bool {
    m == "cn-font-heading"
        || m.starts_with("cn-menu-")
        || m.ends_with("-logical")
        || m == "cn-rtl-flip"
        || m == "cn-logical-sides"
}
