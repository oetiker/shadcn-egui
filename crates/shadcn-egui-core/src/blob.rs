//! Reader for the icon blobs icongen writes (`tools/icongen/src/blob.rs`
//! documents the format). Icon crates hold one blob each; an `Icon` points
//! at its record with a [`BlobRange`].
//!
//! Records start at any even offset of an `include_bytes!` blob, so words
//! are read byte pair by byte pair, never through a `&[u16]` cast. A
//! malformed record must not crash a frame: reads past its end stop the
//! strokes and drop the fill.

use crate::icon::Seg;

const COORD_SCALE: f32 = 1024.0;
const MITER_SCALE: f32 = 8192.0;

/// One icon's record inside an icon crate's blob: `blob[start..start + len]`.
#[derive(Clone, Copy)]
pub struct BlobRange {
    pub blob: &'static [u8],
    pub start: u32,
    pub len: u32,
}

impl BlobRange {
    /// The record's bytes; empty when the range does not fit the blob.
    fn bytes(self) -> &'static [u8] {
        let start = self.start as usize;
        start
            .checked_add(self.len as usize)
            .and_then(|end| self.blob.get(start..end))
            .unwrap_or(&[])
    }
}

/// Two ranges are the same icon when they point into the same blob at the
/// same place; the bytes are never compared.
impl PartialEq for BlobRange {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.blob.as_ptr(), other.blob.as_ptr())
            && self.start == other.start
            && self.len == other.len
    }
}

impl core::fmt::Debug for BlobRange {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlobRange")
            .field("start", &self.start)
            .field("len", &self.len)
            .finish()
    }
}

/// Little-endian 16-bit reader over one record; `None` past its end.
#[derive(Clone)]
struct Reader {
    bytes: &'static [u8],
    pos: usize,
}

impl Reader {
    fn word(&mut self) -> Option<u16> {
        let pair = self.bytes.get(self.pos..self.pos + 2)?;
        self.pos += 2;
        Some(u16::from_le_bytes([pair[0], pair[1]]))
    }
    fn coord(&mut self) -> Option<f32> {
        Some(f32::from(self.word()? as i16) / COORD_SCALE)
    }
    fn miter(&mut self) -> Option<f32> {
        Some(f32::from(self.word()? as i16) / MITER_SCALE)
    }
    fn point(&mut self) -> Option<[f32; 2]> {
        Some([self.coord()?, self.coord()?])
    }
}

/// The stroke commands of one blob record, in the 24-unit box.
#[derive(Clone)]
pub struct BlobSegs {
    /// Reads only the stroke section, so an opcode cut short ends it.
    r: Reader,
}

impl BlobSegs {
    pub(crate) fn new(range: BlobRange) -> Self {
        let bytes = range.bytes();
        let mut r = Reader { bytes, pos: 0 };
        let end = r.word().map_or(0, |len| 2 + 2 * usize::from(len));
        BlobSegs {
            r: Reader {
                bytes: &bytes[..end.min(bytes.len())],
                pos: r.pos,
            },
        }
    }

    fn seg(&mut self) -> Option<Seg> {
        let r = &mut self.r;
        Some(match r.word()? {
            0 => Seg::M(r.coord()?, r.coord()?),
            1 => Seg::L(r.coord()?, r.coord()?),
            2 => Seg::Q(r.coord()?, r.coord()?, r.coord()?, r.coord()?),
            3 => Seg::C(
                r.coord()?,
                r.coord()?,
                r.coord()?,
                r.coord()?,
                r.coord()?,
                r.coord()?,
            ),
            4 => Seg::Z,
            // Not written by icongen.
            _ => return None,
        })
    }
}

impl core::fmt::Debug for BlobSegs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlobSegs")
            .field("pos", &self.r.pos)
            .field("end", &self.r.bytes.len())
            .finish()
    }
}

impl Iterator for BlobSegs {
    type Item = Seg;

    fn next(&mut self) -> Option<Seg> {
        let seg = self.seg();
        if seg.is_none() {
            // A malformed record: stop drawing rather than crash a frame.
            self.r.pos = self.r.bytes.len();
        }
        seg
    }
}

/// A pre-triangulated fill in the 24-unit box, plus per contour each
/// vertex and its outward miter (length ≤ 2; zero where no fringe belongs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FillMesh {
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
    pub contours: Vec<Vec<([f32; 2], [f32; 2])>>,
}

/// The record's fill; `None` for an all-zero fill section and for a
/// malformed one (truncated, or a triangle index past the vertices).
pub(crate) fn fill_mesh(range: BlobRange) -> Option<FillMesh> {
    let mut r = Reader {
        bytes: range.bytes(),
        pos: 0,
    };
    let stroke_len = usize::from(r.word()?);
    r.pos += 2 * stroke_len;
    let (nv, ni, nc) = (
        usize::from(r.word()?),
        usize::from(r.word()?),
        usize::from(r.word()?),
    );
    if nv == 0 && nc == 0 {
        return None;
    }
    let vertices: Vec<[f32; 2]> = (0..nv).map(|_| r.point()).collect::<Option<_>>()?;
    let indices: Vec<u16> = (0..ni).map(|_| r.word()).collect::<Option<_>>()?;
    if ni % 3 != 0 || indices.iter().any(|&i| usize::from(i) >= nv) {
        return None;
    }
    let contours: Vec<Vec<([f32; 2], [f32; 2])>> = (0..nc)
        .map(|_| {
            let len = usize::from(r.word()?);
            (0..len)
                .map(|_| Some((r.point()?, [r.miter()?, r.miter()?])))
                .collect::<Option<_>>()
        })
        .collect::<Option<_>>()?;
    Some(FillMesh {
        vertices,
        indices,
        contours,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::{Icon, IconShape, Seg};

    /// Words (as i16 values, so miters can be negative) to leaked LE bytes.
    fn leak(words: &[i32]) -> &'static [u8] {
        let bytes: Vec<u8> = words
            .iter()
            .flat_map(|&w| (w as i16 as u16).to_le_bytes())
            .collect();
        Box::leak(bytes.into_boxed_slice())
    }

    /// M(1, 2) L(3, 4) Z, then a one-triangle fill with one contour.
    fn record() -> &'static [u8] {
        leak(&[
            7, 0, 1024, 2048, 1, 3072, 4096, 4, // stroke
            3, 3, 1, // nv ni nc
            0, 0, 1024, 0, 0, 1024, // vertices
            0, 1, 2, // indices
            3, 0, 0, -8192, -8192, 1024, 0, 8192, -8192, 0, 1024, -8192, 8192, // contour
        ])
    }

    fn blob_icon(blob: &'static [u8], start: u32, len: u32) -> Icon {
        Icon {
            name: "t",
            size: 24.0,
            shape: IconShape::Blob(BlobRange { blob, start, len }),
        }
    }

    #[test]
    fn blob_record_decodes_strokes_and_fill() {
        let blob = record();
        let icon = blob_icon(blob, 0, blob.len() as u32);
        assert_eq!(
            icon.segs().collect::<Vec<_>>(),
            vec![Seg::M(1.0, 2.0), Seg::L(3.0, 4.0), Seg::Z]
        );
        let fill = icon.fill_mesh().expect("has a fill");
        assert_eq!(fill.vertices, vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
        assert_eq!(fill.indices, vec![0, 1, 2]);
        assert_eq!(fill.contours.len(), 1);
        assert_eq!(fill.contours[0][0], ([0.0, 0.0], [-1.0, -1.0]));
        assert_eq!(fill.contours[0][1], ([1.0, 0.0], [1.0, -1.0]));
    }

    #[test]
    fn records_are_read_at_their_offset() {
        // M(1, 1) with no fill (7 words = 14 bytes), then Z with no fill.
        let blob = leak(&[3, 0, 1024, 1024, 0, 0, 0, 1, 4, 0, 0, 0]);
        let second = blob_icon(blob, 14, 10);
        assert_eq!(second.segs().collect::<Vec<_>>(), vec![Seg::Z]);
        assert_eq!(
            second.fill_mesh(),
            None,
            "an all-zero fill section is no fill"
        );
        assert_eq!(
            blob_icon(blob, 0, 14).segs().collect::<Vec<_>>(),
            vec![Seg::M(1.0, 1.0)]
        );
    }

    #[test]
    fn segs_icons_iterate_their_slice_and_have_no_fill() {
        static SEGS: [Seg; 2] = [Seg::M(0.0, 0.0), Seg::L(1.0, 1.0)];
        let icon = Icon {
            name: "s",
            size: 24.0,
            shape: IconShape::Segs(&SEGS),
        };
        assert_eq!(icon.segs().collect::<Vec<_>>(), SEGS.to_vec());
        assert_eq!(icon.fill_mesh(), None);
    }

    #[test]
    fn unknown_opcode_ends_the_strokes_instead_of_panicking() {
        let blob = leak(&[3, 9, 0, 0, 0, 0, 0]);
        assert_eq!(blob_icon(blob, 0, 14).segs().count(), 0);
    }

    #[test]
    fn truncated_and_garbage_records_never_panic() {
        let whole = record();
        // Every prefix of a valid record, including odd lengths.
        for len in 0..=whole.len() as u32 {
            let icon = blob_icon(whole, 0, len);
            let _ = icon.segs().count();
            let _ = icon.fill_mesh();
        }
        // Ranges that point past the end of the blob.
        for (start, len) in [(0, 1000), (40, 20), (u32::MAX, 2), (2, u32::MAX)] {
            let icon = blob_icon(whole, start, len);
            let _ = icon.segs().count();
            let _ = icon.fill_mesh();
        }
        // Fixed garbage: huge counts, odd lengths, opcodes cut short.
        let garbage: [&[i32]; 6] = [
            &[-1],
            &[-1, 0, 0],
            &[2, 3, 0],
            &[0, -1, -1, -1, 0, 0],
            &[0, 1, 0, 0, 7, 0x7fff, 0, 0],
            &[1, 4, 0, 0, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0],
        ];
        for words in garbage {
            let blob = leak(words);
            for len in [blob.len() as u32, blob.len() as u32 - 1] {
                let icon = blob_icon(blob, 0, len);
                assert!(icon.segs().count() <= blob.len());
                let _ = icon.fill_mesh();
            }
        }
    }

    #[test]
    fn a_triangle_index_past_the_vertices_drops_the_fill() {
        // One vertex, one triangle 0 1 2, no contours.
        let blob = leak(&[0, 1, 3, 0, 0, 0, 0, 1, 2]);
        assert_eq!(blob_icon(blob, 0, blob.len() as u32).fill_mesh(), None);
    }

    #[test]
    fn blob_ranges_compare_by_identity_and_print_without_bytes() {
        let blob = record();
        let a = BlobRange {
            blob,
            start: 0,
            len: 8,
        };
        assert_eq!(a, a);
        assert_ne!(a, BlobRange { start: 2, ..a });
        let copy: &'static [u8] = Box::leak(blob.to_vec().into_boxed_slice());
        assert_ne!(
            a,
            BlobRange { blob: copy, ..a },
            "equal bytes in another blob are another icon"
        );
        assert_eq!(format!("{a:?}"), "BlobRange { start: 0, len: 8 }");
    }
}
