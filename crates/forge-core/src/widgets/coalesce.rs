//! Dirty-rect coalescing for the desktop widget engines.
//!
//! Both engines accumulate framebuffer updates here instead of sending one
//! wire frame per decoded update: rects merge while the transport is busy, and
//! payloads are re-sliced from the latest framebuffer at flush time — so a
//! slow client receives the current state in a bounded number of frames
//! instead of a growing queue of stale ones. Union-merging is safe w.r.t.
//! overlapping-update order for the same reason: the framebuffer already
//! reflects every later overwrite.

/// One framebuffer rectangle, in the wire frame's coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    fn right(&self) -> u32 {
        u32::from(self.x) + u32::from(self.w)
    }

    fn bottom(&self) -> u32 {
        u32::from(self.y) + u32::from(self.h)
    }

    fn area(&self) -> u64 {
        u64::from(self.w) * u64::from(self.h)
    }

    fn contains(&self, other: &Rect) -> bool {
        self.x <= other.x
            && self.y <= other.y
            && self.right() >= other.right()
            && self.bottom() >= other.bottom()
    }

    fn intersects(&self, other: &Rect) -> bool {
        u32::from(self.x) < other.right()
            && u32::from(other.x) < self.right()
            && u32::from(self.y) < other.bottom()
            && u32::from(other.y) < self.bottom()
    }

    fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        // Coordinates are u16 and rects are in-bounds, so the union edges fit.
        #[allow(clippy::cast_possible_truncation)]
        Rect {
            x,
            y,
            w: (self.right().max(other.right()) - u32::from(x)) as u16,
            h: (self.bottom().max(other.bottom()) - u32::from(y)) as u16,
        }
    }
}

/// Merged dirty-region list. Bounded: a flood of updates degrades into a
/// single bounding box, never an unbounded queue.
#[derive(Debug, Default)]
pub struct DirtyRegion {
    rects: Vec<Rect>,
}

impl DirtyRegion {
    /// Cap on tracked rects before collapsing to one bounding box.
    pub const MAX_RECTS: usize = 16;

    /// Merge one dirty rect in. Overlapping rects union; nearby rects union
    /// when the merged box wastes less than ~30% of its area; rects the new
    /// one fully covers are dropped.
    pub fn add(&mut self, mut new: Rect) {
        if new.w == 0 || new.h == 0 {
            return;
        }
        self.rects.retain(|r| !new.contains(r));
        loop {
            let merge = self.rects.iter().position(|r| {
                r.contains(&new)
                    || r.intersects(&new)
                    || r.union(&new).area() * 10 <= (r.area() + new.area()) * 13
            });
            match merge {
                // The union may now touch rects it previously didn't: re-scan.
                Some(i) => new = self.rects.swap_remove(i).union(&new),
                None => break,
            }
        }
        self.rects.push(new);
        if self.rects.len() > Self::MAX_RECTS {
            let all = self
                .rects
                .drain(..)
                .reduce(|a, b| a.union(&b))
                .expect("over MAX_RECTS implies non-empty");
            self.rects.push(all);
        }
    }

    /// Take the merged rects, leaving the region empty.
    pub fn take(&mut self) -> Vec<Rect> {
        std::mem::take(&mut self.rects)
    }

    pub fn clear(&mut self) {
        self.rects.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }
}

/// Latest-state RGBA framebuffer (stride = `width * 4`). The VNC engine's
/// source of truth for flush-time re-slicing; RDP re-slices IronRDP's
/// `DecodedImage` instead.
pub struct Framebuffer {
    width: u16,
    height: u16,
    pixels: Vec<u8>,
}

impl Framebuffer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; usize::from(width) * usize::from(height) * 4],
        }
    }

    /// Copy one update rect in. `false` = the rect falls outside the
    /// framebuffer or the payload length doesn't match — dropped.
    pub fn blit(&mut self, r: Rect, rgba: &[u8]) -> bool {
        let row_len = usize::from(r.w) * 4;
        if r.right() > u32::from(self.width)
            || r.bottom() > u32::from(self.height)
            || rgba.len() != row_len * usize::from(r.h)
        {
            return false;
        }
        let stride = usize::from(self.width) * 4;
        for (i, row) in rgba.chunks_exact(row_len).enumerate() {
            let start = (usize::from(r.y) + i) * stride + usize::from(r.x) * 4;
            self.pixels[start..start + row_len].copy_from_slice(row);
        }
        true
    }

    /// Copy one rect out, row-major. The rect must be in bounds (only rects
    /// previously accepted by [`Self::blit`] reach this).
    pub fn slice(&self, r: Rect) -> Vec<u8> {
        debug_assert!(r.right() <= u32::from(self.width));
        debug_assert!(r.bottom() <= u32::from(self.height));
        let row_len = usize::from(r.w) * 4;
        let stride = usize::from(self.width) * 4;
        let mut out = Vec::with_capacity(row_len * usize::from(r.h));
        for row in 0..usize::from(r.h) {
            let start = (usize::from(r.y) + row) * stride + usize::from(r.x) * 4;
            out.extend_from_slice(&self.pixels[start..start + row_len]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn contained_rects_are_dropped() {
        let mut dirty = DirtyRegion::default();
        dirty.add(rect(10, 10, 4, 4));
        dirty.add(rect(0, 0, 100, 100));
        assert_eq!(dirty.take(), vec![rect(0, 0, 100, 100)]);
    }

    #[test]
    fn add_into_existing_container_is_absorbed() {
        let mut dirty = DirtyRegion::default();
        dirty.add(rect(0, 0, 100, 100));
        dirty.add(rect(10, 10, 4, 4));
        assert_eq!(dirty.take(), vec![rect(0, 0, 100, 100)]);
    }

    #[test]
    fn overlapping_rects_union() {
        let mut dirty = DirtyRegion::default();
        dirty.add(rect(0, 0, 10, 10));
        dirty.add(rect(5, 5, 10, 10));
        assert_eq!(dirty.take(), vec![rect(0, 0, 15, 15)]);
    }

    #[test]
    fn distant_rects_stay_separate() {
        let mut dirty = DirtyRegion::default();
        dirty.add(rect(0, 0, 10, 10));
        dirty.add(rect(500, 500, 10, 10));
        let rects = dirty.take();
        assert_eq!(rects.len(), 2);
    }

    #[test]
    fn adjacent_rects_merge_when_union_is_tight() {
        // Two abutting halves of a row: the union wastes nothing.
        let mut dirty = DirtyRegion::default();
        dirty.add(rect(0, 0, 10, 10));
        dirty.add(rect(10, 0, 10, 10));
        assert_eq!(dirty.take(), vec![rect(0, 0, 20, 10)]);
    }

    #[test]
    fn merge_cascades_through_touched_neighbours() {
        // Third rect bridges the two separate ones: all collapse to one.
        let mut dirty = DirtyRegion::default();
        dirty.add(rect(0, 0, 10, 10));
        dirty.add(rect(20, 0, 10, 10));
        assert_eq!(dirty.rects.len(), 2);
        dirty.add(rect(5, 0, 20, 10));
        assert_eq!(dirty.take(), vec![rect(0, 0, 30, 10)]);
    }

    #[test]
    fn overflow_collapses_to_bounding_box() {
        let mut dirty = DirtyRegion::default();
        for i in 0..=DirtyRegion::MAX_RECTS as u16 {
            dirty.add(rect(i * 100, i * 50, 10, 10));
        }
        let rects = dirty.take();
        assert_eq!(rects.len(), 1);
        let max = DirtyRegion::MAX_RECTS as u16;
        assert_eq!(rects[0], rect(0, 0, max * 100 + 10, max * 50 + 10));
    }

    #[test]
    fn zero_sized_rects_are_ignored() {
        let mut dirty = DirtyRegion::default();
        dirty.add(rect(5, 5, 0, 10));
        dirty.add(rect(5, 5, 10, 0));
        assert!(dirty.is_empty());
    }

    #[test]
    fn take_empties_the_region() {
        let mut dirty = DirtyRegion::default();
        dirty.add(rect(0, 0, 10, 10));
        assert!(!dirty.is_empty());
        assert_eq!(dirty.take().len(), 1);
        assert!(dirty.is_empty());
        assert!(dirty.take().is_empty());
    }

    #[test]
    fn framebuffer_blit_slice_roundtrip() {
        let mut fb = Framebuffer::new(8, 8);
        let r = rect(2, 3, 3, 2);
        let rgba: Vec<u8> = (0..3 * 2 * 4).map(|i| i as u8).collect();
        assert!(fb.blit(r, &rgba));
        assert_eq!(fb.slice(r), rgba);
        // Neighbouring pixel untouched.
        assert_eq!(fb.slice(rect(1, 3, 1, 1)), vec![0; 4]);
    }

    #[test]
    fn framebuffer_rejects_out_of_bounds_and_bad_lengths() {
        let mut fb = Framebuffer::new(8, 8);
        assert!(!fb.blit(rect(7, 0, 2, 1), &[0; 8])); // right edge overflow
        assert!(!fb.blit(rect(0, 7, 1, 2), &[0; 8])); // bottom edge overflow
        assert!(!fb.blit(rect(0, 0, 2, 2), &[0; 8])); // payload too short
    }
}
