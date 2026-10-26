//! `text-wrap: balance` and `text-wrap: pretty` as Blink breaks them
//! (Chromium 156, `third_party/blink/renderer/core/layout/inline/`):
//! `ScoreLineBreaker` (Knuth-Plass scores with Minikin's penalties) for
//! both, `ParagraphLineBreaker`'s bisection of the available width as the
//! fallback for `balance`. Positions, widths and font sizes are in device
//! px, as Blink holds them (zoomed px), so that the float sums and the
//! `LayoutUnit` snapping fall as in Blink; `zoom` is the device pixels per
//! point.

/// A break opportunity: the next line starts at char `start` at
/// `pos_no_break` (after the spaces). The line before it ends at
/// `pos_if_break` for the score breaker: its last glyph shaped alone, as
/// Blink's `AppendCandidates` reshapes a word end that is not safe to
/// break, trailing spaces hanging; and at `pos_greedy` for the greedy
/// breaker, which breaks after a run of spaces and keeps the kerning
/// against the first space inside the line (elsewhere it reshapes the end
/// too). Blink's `LineBreakCandidate`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Candidate {
    pub start: usize,
    pub pos_no_break: f32,
    pub pos_if_break: f32,
    pub pos_greedy: f32,
    pub hyphenated: bool,
}

/// One char of a paragraph as the breakers see it, in device px: its
/// glyph's left edge (after the kerning against the char before), its
/// right edge (before the kerning against the next), as Blink's
/// `ShapeResult` gives them (`LayoutUnit`s, a kerned right edge shaped
/// alone).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Glyph {
    pub c: char,
    pub left: f32,
    pub right: f32,
}

/// `kMaxLinesForBalance`, `kMaxLinesForOptimal`.
const MAX_LINES_BALANCE: usize = 6;
const MAX_LINES_PRETTY: usize = 4;
const SCORE_INFINITY: f32 = f32::MAX;
const SCORE_OVERFULL: f32 = 1e12;
const LAST_LINE_PENALTY_MULTIPLIER: f32 = 4.0; // behaviour-ok: kLastLinePenaltyMultiplier
const ORPHANS_PENALTY: f32 = 10000.0; // behaviour-ok: kOrphansPenalty
/// `ShouldOptimize`: a last line under a third of the width is short.
const SHORT_LINE_DENOMINATOR: f32 = 3.0; // behaviour-ok: kShortLineDenominator
/// `AttemptParagraphBalancing` starts bisecting at 80% of the average line
/// width ("20% buffer for potential edge cases").
const BALANCE_START: f32 = 0.8; // behaviour-ok: Blink's bisection start

/// `LayoutUnit`s per device px.
const LAYOUT_UNITS: f32 = 64.0; // behaviour-ok: LayoutUnit::kFixedPointDenominator
/// Float noise allowed when a box width snaps to the `LayoutUnit` grid, in
/// `LayoutUnit`s: the caller's width times the zoom may land a hair under
/// a grid line that Blink's own length lands on.
const SNAP_TOLERANCE: f32 = 1e-3;

/// One `LayoutUnit` in device px.
fn unit() -> f32 {
    LAYOUT_UNITS.recip()
}

/// The box width `width` as Blink holds it: truncated to the grid.
fn snap_width(width: f32) -> f32 {
    (width * LAYOUT_UNITS + SNAP_TOLERANCE).floor() / LAYOUT_UNITS
}

/// A line's shaped width as Blink holds it: rounded up to the grid
/// (`ShapeResult::SnappedWidth`).
fn snap_len(len: f32) -> f32 {
    (len * LAYOUT_UNITS).ceil() / LAYOUT_UNITS
}

/// Whether a line `len` long fits `width`: its snapped width may pass the
/// box width by one unit (`AvailableWidthToFit`).
fn fits(len: f32, width: f32) -> bool {
    snap_len(len) <= snap_width(width) + unit()
}

/// Whether Blink breaks spaces off a line (`IsBreakableSpace`).
fn is_space(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// The break candidates of one paragraph (no forced break inside): a
/// sentinel at the start, one after each run of spaces followed by text,
/// one after a hyphen between a non-space and a non-digit (UAX #14 HY),
/// and a sentinel at the end of the last char that is not a space.
pub(super) fn candidates(chars: &[Glyph]) -> Vec<Candidate> {
    let x0 = chars.first().map_or(0.0, |c| c.left);
    let mut out = vec![Candidate {
        start: 0,
        pos_no_break: x0,
        pos_if_break: x0,
        pos_greedy: x0,
        hyphenated: false,
    }];
    let mut end = x0;
    let mut i = 0;
    while i < chars.len() {
        let g = chars[i];
        if is_space(g.c) {
            let mut j = i;
            while j < chars.len() && is_space(chars[j].c) {
                j += 1;
            }
            if i > 0 && j < chars.len() {
                out.push(Candidate {
                    start: j,
                    pos_no_break: chars[j].left,
                    pos_if_break: chars[i - 1].right,
                    pos_greedy: g.left,
                    hyphenated: false,
                });
            }
            i = j;
            continue;
        }
        end = g.right;
        let next = chars
            .get(i + 1)
            .filter(|n| !is_space(n.c) && !n.c.is_ascii_digit());
        if let Some(next) = next.filter(|_| g.c == '-' && i > 0 && !is_space(chars[i - 1].c)) {
            out.push(Candidate {
                start: i + 1,
                pos_no_break: next.left,
                pos_if_break: g.right,
                pos_greedy: g.right,
                hyphenated: false,
            });
        }
        i += 1;
    }
    out.push(Candidate {
        start: chars.len(),
        pos_no_break: end,
        pos_if_break: end,
        pos_greedy: end,
        hyphenated: false,
    });
    out
}

/// Blink's greedy `LineBreaker` over `cands` from candidate `from`: each
/// line ends at the last candidate that fits `width` (`fits`). The candidate index ending each line, or `None` where a
/// line overflows (Blink then disables both the score and the bisect
/// breaker) or the paragraph needs more than `max_lines`.
fn greedy(cands: &[Candidate], from: usize, width: f32, max_lines: usize) -> Option<Vec<usize>> {
    let mut ends = Vec::new();
    let mut s = from;
    while s + 1 < cands.len() {
        let start = cands[s].pos_no_break;
        let e = (s + 1..cands.len())
            .take_while(|&e| fits(cands[e].pos_greedy - start, width))
            .last()?;
        ends.push(e);
        if ends.len() > max_lines {
            return None;
        }
        s = e;
    }
    Some(ends)
}

/// Blink's normal line breaking (`text-wrap: wrap`) over `cands` at
/// `width` (points): each line ends at the last candidate that fits, its
/// trailing spaces hanging; a line where none fits takes the first, its
/// word overflowing (`overflow-wrap: normal`). The char index where each
/// line after the first starts.
pub(super) fn normal(cands: &[Candidate], width: f32) -> Vec<usize> {
    let mut out = Vec::new();
    let mut s = 0;
    while s + 1 < cands.len() {
        let start = cands[s].pos_no_break;
        let e = (s + 1..cands.len())
            .take_while(|&e| fits(cands[e].pos_greedy - start, width))
            .last()
            .unwrap_or(s + 1);
        if e + 1 < cands.len() {
            out.push(cands[e].start);
        }
        s = e;
    }
    out
}

/// `ScoreLineBreaker::Optimize` for the paragraph part `cands` (its first
/// candidate is the start of its first line): the candidate indices ending
/// each line, or `None` where Blink gives up (fewer than three break
/// opportunities, or another line count than `lines`).
// `active` moves for the next `end`, as in Blink; the running loop keeps
// its range.
#[allow(clippy::mut_range_bound)]
fn score(
    cands: &[Candidate],
    width: f32,
    zoom: f32,
    font_size: f32,
    balanced: bool,
    lines: usize,
) -> Option<Vec<usize>> {
    if cands.len() < 3 + 2 {
        return None;
    }
    // `SetupParameters`, not justified.
    let width = snap_width(width.max(0.0));
    let hyphen_penalty = width * font_size / zoom * 2.0;
    let line_penalty = hyphen_penalty * 2.0;
    let mut penalty: Vec<f32> = cands
        .iter()
        .map(|c| if c.hyphenated { hyphen_penalty } else { 0.0 })
        .collect();
    // Typographic orphans: the last break opportunity costs more, and so
    // does each hyphenated one right before it.
    for k in (0..cands.len() - 1).rev() {
        penalty[k] += ORPHANS_PENALTY * zoom;
        if !cands[k].hyphenated {
            break;
        }
    }
    let fit = width + unit();
    // (score, previous index, line index) per candidate.
    let mut scores: Vec<(f32, usize, usize)> = vec![(0.0, 0, 0)];
    let mut active = 0;
    for end in 1..cands.len() {
        let end_c = cands[end];
        let is_last = end == cands.len() - 1;
        let mut best = SCORE_INFINITY;
        let mut best_prev = 0;
        let start_edge = end_c.pos_if_break - fit;
        let mut best_hope = 0.0;
        for start in active..end {
            let start_score = scores[start].0;
            if start_score + best_hope >= best {
                continue;
            }
            let delta = cands[start].pos_no_break - start_edge;
            let mut width_score = 0.0;
            let mut additional = 0.0;
            if delta < 0.0 {
                width_score = SCORE_OVERFULL;
            } else if is_last && !balanced {
                additional = LAST_LINE_PENALTY_MULTIPLIER * penalty[start];
            } else {
                width_score = delta * delta / zoom;
            }
            if delta < 0.0 {
                active = start + 1;
            } else {
                best_hope = width_score;
            }
            let s = start_score + width_score + additional;
            if s <= best {
                best = s;
                best_prev = start;
            }
        }
        scores.push((
            best + penalty[end] + line_penalty,
            best_prev,
            scores[best_prev].2 + 1,
        ));
    }
    let mut ends = Vec::new();
    let mut i = cands.len() - 1;
    while i > 0 {
        ends.push(i);
        i = scores[i].1;
    }
    ends.reverse();
    (ends.len() == lines).then_some(ends)
}

/// `ParagraphLineBreaker::AttemptParagraphBalancing`: the narrowest width
/// (bisected to `epsilon`, in `LayoutUnit` steps) at which the
/// greedy breaker still needs the lines it needs at `width`, and the line
/// ends there. `estimated` is Blink's `EstimateNumLines` guess.
fn bisect(cands: &[Candidate], width: f32, epsilon: f32, estimated: usize) -> Option<Vec<usize>> {
    if estimated > MAX_LINES_BALANCE * 2 || width <= 0.0 {
        return None;
    }
    let normal = greedy(cands, 0, width, MAX_LINES_BALANCE)?;
    let lines = normal.len();
    if lines <= 1 {
        return None;
    }
    // `LineWidthSum() / num_lines` in `LayoutUnit`s, then rounded.
    let sum: f32 = line_widths(cands, 0, &normal).map(snap_len).sum();
    let average = (sum * LAYOUT_UNITS / lines as f32).floor() / LAYOUT_UNITS;
    let mut lower = (average * BALANCE_START * LAYOUT_UNITS).round() / LAYOUT_UNITS;
    let mut upper = snap_width(width);
    while lower + epsilon < upper {
        let middle = snap_width((upper + lower) / 2.0);
        match greedy(cands, 0, middle, lines) {
            Some(ends) if ends.len() <= lines => upper = middle,
            _ => lower = middle,
        }
    }
    greedy(cands, 0, upper, lines)
}

/// The width of each greedy line ending at `ends`, the first starting at
/// `from` (Blink's `LineInfo` widths).
fn line_widths<'a>(
    cands: &'a [Candidate],
    from: usize,
    ends: &'a [usize],
) -> impl Iterator<Item = f32> + 'a {
    std::iter::once(from)
        .chain(ends.iter().copied())
        .zip(ends.iter().copied())
        .map(|(s, e)| cands[e].pos_greedy - cands[s].pos_no_break)
}

/// How a paragraph is broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Style {
    Balance,
    Pretty,
}

/// The char index where each line after the first starts, for one
/// paragraph under `style` at `width` (device px, as `font_size`);
/// `None` where Blink keeps its greedy breaks. `zoom` is the device pixels
/// per point: Blink unzooms its scores once, and the bisection stops at
/// one CSS pixel rounded up to whole device pixels. `space_width` is the
/// primary font's space advance and `text_len` the block's text length in
/// chars, for Blink's line estimate.
#[allow(clippy::too_many_arguments)]
pub(super) fn breaks(
    cands: &[Candidate],
    style: Style,
    width: f32,
    font_size: f32,
    zoom: f32,
    space_width: f32,
    text_len: usize,
) -> Option<Vec<usize>> {
    let starts = |ends: Vec<usize>, from: usize| {
        let n = ends.len();
        ends[..n - 1]
            .iter()
            .map(|&e| cands[from + e].start)
            .collect::<Vec<_>>()
    };
    match style {
        Style::Balance => {
            let normal = greedy(cands, 0, width, MAX_LINES_BALANCE)?;
            if normal.len() <= 1 {
                return None;
            }
            let ends = score(cands, width, zoom, font_size, true, normal.len()).or_else(|| {
                let per_line = if space_width > 0.0 {
                    (width / space_width) as usize
                } else {
                    0
                };
                let estimated = match per_line {
                    _ if space_width <= 0.0 => 0,
                    0 => usize::MAX,
                    n => text_len.div_ceil(n),
                };
                let epsilon = zoom.ceil();
                bisect(cands, width, epsilon, estimated)
            })?;
            Some(starts(ends, 0))
        }
        Style::Pretty => {
            let normal = greedy(cands, 0, width, usize::MAX)?;
            let n = normal.len();
            if n <= 1 {
                return None;
            }
            // Blink scores the last lines that fit its line cache.
            let k = n.saturating_sub(MAX_LINES_PRETTY);
            let from = if k == 0 { 0 } else { normal[k - 1] };
            let last_start = normal.get(n.wrapping_sub(2)).copied().unwrap_or(0);
            let last_width = snap_len(line_widths(cands, from, &normal[k..]).last()?);
            // `ShouldOptimize`: a short last line holding one word; the
            // third of the width in whole `LayoutUnit`s.
            let third =
                (snap_width(width) * LAYOUT_UNITS / SHORT_LINE_DENOMINATOR).floor() / LAYOUT_UNITS;
            if !(last_width < third && last_start == cands.len() - 2) {
                return None;
            }
            let window = &cands[from..];
            let ends = score(window, width, zoom, font_size, false, n - k)?;
            let mut out: Vec<usize> = normal[..k].iter().map(|&e| cands[e].start).collect();
            out.extend(starts(ends, from));
            Some(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Monospace text: every char 1 wide.
    fn mono(text: &str) -> Vec<Glyph> {
        text.chars()
            .enumerate()
            .map(|(i, c)| Glyph {
                c,
                left: i as f32,
                right: i as f32 + 1.0,
            })
            .collect()
    }

    fn lines(text: &str, style: Style, width: f32) -> Vec<String> {
        let chars = mono(text);
        let cands = candidates(&chars);
        let starts = breaks(&cands, style, width, 14.0, 1.0, 1.0, chars.len()).unwrap_or_default();
        let mut out = Vec::new();
        let mut s = 0;
        for e in starts.into_iter().chain(std::iter::once(chars.len())) {
            out.push(
                text.chars()
                    .skip(s)
                    .take(e - s)
                    .collect::<String>()
                    .trim_end()
                    .to_owned(),
            );
            s = e;
        }
        out
    }

    #[test]
    fn candidates_sit_at_spaces_and_hyphens() {
        let c = candidates(&mono("ab cd-ef  g-1 "));
        let starts: Vec<usize> = c.iter().map(|c| c.start).collect();
        assert_eq!(starts, [0, 3, 6, 10, 14]);
        assert_eq!((c[1].pos_if_break, c[1].pos_no_break), (2.0, 3.0));
        assert_eq!((c[2].pos_if_break, c[2].pos_no_break), (6.0, 6.0));
        assert_eq!(c[4].pos_if_break, 13.0);
    }

    #[test]
    fn greedy_lines_keep_the_kerning_against_the_space_the_score_breaker_not() {
        // "ab cd ef" with the second space kerned 0.25 closer to "d".
        let mut chars = mono("ab cd ef");
        for g in &mut chars[5..] {
            g.left -= 0.25;
            g.right -= 0.25;
        }
        let c = candidates(&chars);
        assert_eq!((c[2].pos_greedy, c[2].pos_if_break), (4.75, 5.0));
        // "ab cd" ends at 4.75 kerned: it fits 4.8 for the greedy breaker.
        assert_eq!(normal(&c, 4.8), [6]);
        assert_eq!(normal(&c, 4.7), [3, 6]);
    }

    #[test]
    fn balance_evens_the_lines_and_keeps_their_count() {
        let text = "aaaa bbbb cccc dddd eeee ffff gg";
        assert_eq!(
            lines(text, Style::Balance, 25.0),
            ["aaaa bbbb cccc", "dddd eeee ffff gg"]
        );
    }

    #[test]
    fn balance_gives_up_past_six_lines_and_on_one_line() {
        let text = "aa bb cc dd ee ff gg hh ii jj kk ll mm nn";
        assert_eq!(lines(text, Style::Balance, 5.0).len(), 1);
        assert_eq!(lines("aa bb", Style::Balance, 50.0).len(), 1);
    }

    #[test]
    fn pretty_moves_a_word_down_to_a_lone_last_word() {
        let text = "aaaa bbbb cccc dddd e";
        assert_eq!(
            lines(text, Style::Pretty, 19.0),
            ["aaaa bbbb cccc", "dddd e"]
        );
        // A last line of two words is left alone.
        assert_eq!(lines("aaaa bbbb cccc dd ee", Style::Pretty, 15.0).len(), 1);
    }

    #[test]
    fn an_overflowing_word_keeps_greedy_breaks() {
        let text = "aaaaaaaaaaaaaaaaaaaa b c d e";
        assert_eq!(lines(text, Style::Balance, 10.0).len(), 1);
    }
}
