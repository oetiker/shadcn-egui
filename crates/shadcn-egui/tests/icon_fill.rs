#[allow(dead_code)]
#[path = "icons/fixture/src/lib.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "icons/lucide.rs"]
#[rustfmt::skip]
mod lucide;

use shadcn_egui::{Icon, Seg};

fn coords(s: Seg) -> Vec<f32> {
    match s {
        Seg::M(x, y) | Seg::L(x, y) => vec![x, y],
        Seg::Q(a, b, x, y) => vec![a, b, x, y],
        Seg::C(a, b, c, d, x, y) => vec![a, b, c, d, x, y],
        Seg::Z => vec![],
    }
}

fn area(f: &shadcn_egui::FillMesh) -> f32 {
    f.indices
        .chunks(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| f.vertices[i as usize]);
            ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
        })
        .sum()
}

#[test]
fn blob_strokes_match_the_slice_form() {
    let blob: Vec<Seg> = fixture::X.segs().collect();
    let slice: Vec<Seg> = lucide::X.segs().collect();
    assert_eq!(blob.len(), slice.len());
    for (b, s) in blob.iter().zip(&slice) {
        for (x, y) in coords(*b).iter().zip(coords(*s)) {
            assert!((x - y).abs() <= 0.5 / 1024.0, "{b:?} vs {s:?}");
        }
    }
    assert_eq!(fixture::X.fill_mesh(), None, "a stroke icon has no fill");
}

#[test]
fn blob_fills_decode_with_their_area_and_contours() {
    let f = fixture::SQUARE_HOLE.fill_mesh().expect("filled");
    assert!((area(&f) - 220.0).abs() < 0.01, "area {}", area(&f));
    assert_eq!(f.contours.len(), 2);
    assert!(f.indices.iter().all(|&i| usize::from(i) < f.vertices.len()));
    let mixed = fixture::MIXED;
    assert!(
        mixed.segs().count() > 0 && mixed.fill_mesh().is_some(),
        "strokes and fill in one record"
    );
    let xs: Vec<f32> = fixture::BIG_BOX
        .fill_mesh()
        .expect("filled")
        .vertices
        .iter()
        .flat_map(|v| *v)
        .collect();
    assert!(
        xs.iter().all(|v| (3.0..=21.0).contains(v)),
        "256 view box normalised to 24 units"
    );
}

#[test]
fn generated_consts_point_into_one_blob() {
    assert_eq!(fixture::ALL.len(), 6);
    assert_eq!(fixture::SLOTS.check, fixture::MIXED);
    assert_eq!(fixture::SLOTS.close.name, "x");
    let icon: &Icon = fixture::ALL[0];
    assert_eq!(icon.size, 24.0);
}
