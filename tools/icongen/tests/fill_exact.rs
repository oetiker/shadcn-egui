//! The exact reference (`common::exact`) on shapes whose coverage is known
//! (squares, a 45° edge, overlaps, holes, abutting paths).

mod common;

use common::exact;
use icongen::{FillPath, FillRule};

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<[f32; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn cov(paths: &[FillPath], side: usize) -> Vec<u8> {
    exact::coverage(&exact::sides(paths), side)
}

#[test]
fn exact_coverage_of_known_shapes() {
    // 24 px: one unit per pixel. A square from 2.25 to 5.5 in x and y.
    let sq = FillPath {
        rule: FillRule::NonZero,
        contours: vec![rect(2.25, 2.25, 5.5, 5.5)],
    };
    let c = cov(std::slice::from_ref(&sq), 24);
    assert_eq!(c[2 * 24 + 2], (0.75f64 * 0.75 * 255.0).round() as u8);
    assert_eq!(c[3 * 24 + 3], 255);
    assert_eq!(c[2 * 24 + 5], (0.75f64 * 0.5 * 255.0).round() as u8);
    assert_eq!(c[6 * 24 + 6], 0);
    // A 45° half-plane through pixel corners: the diagonal pixels are half covered.
    let tri = FillPath {
        rule: FillRule::NonZero,
        contours: vec![vec![[0.0, 0.0], [24.0, 24.0], [0.0, 24.0]]],
    };
    let c = cov(&[tri], 24);
    assert!((0..24).all(|i| c[i * 24 + i] == 128));
    assert_eq!(c[5 * 24 + 4], 255);
    assert_eq!(c[4 * 24 + 5], 0);
    // Overlap (winding 2 under nonzero) counts once; a reversed inner square
    // cuts a hole, as does an even-odd ring drawn in one direction.
    let two = FillPath {
        rule: FillRule::NonZero,
        contours: vec![rect(2.0, 2.0, 10.0, 10.0), rect(4.5, 4.5, 12.0, 12.0)],
    };
    let c = cov(&[two], 24);
    assert_eq!(c[5 * 24 + 5], 255);
    assert_eq!(c[4 * 24 + 4], 255);
    let mut inner = rect(4.0, 4.0, 8.0, 8.0);
    inner.reverse();
    let hole = FillPath {
        rule: FillRule::NonZero,
        contours: vec![rect(2.0, 2.0, 10.0, 10.0), inner],
    };
    let eo = FillPath {
        rule: FillRule::EvenOdd,
        contours: vec![rect(2.0, 2.0, 10.0, 10.0), rect(4.0, 4.0, 8.0, 8.0)],
    };
    for p in [hole, eo] {
        let c = cov(&[p], 24);
        assert_eq!(c[5 * 24 + 5], 0);
        assert_eq!(c[3 * 24 + 3], 255);
    }
    // Two abutting squares: no seam, and the union of two paths.
    let a = FillPath {
        rule: FillRule::NonZero,
        contours: vec![rect(2.0, 2.0, 6.3, 10.0)],
    };
    let b = FillPath {
        rule: FillRule::EvenOdd,
        contours: vec![rect(6.3, 2.0, 10.0, 10.0)],
    };
    let c = cov(&[a, b], 24);
    assert!((2..10).all(|x| c[5 * 24 + x] == 255));
}
