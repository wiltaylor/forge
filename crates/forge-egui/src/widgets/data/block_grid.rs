//! Named-region layout helper: a single row of weighted regions that wraps
//! into more rows when the available width can't honour every region's
//! minimum width (simple greedy packing, forge-tui's `BlockGrid` adapted to
//! a continuous canvas).

use egui::{Rect, Ui, UiBuilder, Vec2};

/// One named region: minimum width and a flex weight.
#[derive(Clone, Copy, Debug)]
pub struct BlockSpec<'a> {
    pub name: &'a str,
    pub min_width: f32,
    pub weight: f32,
}

impl<'a> BlockSpec<'a> {
    pub fn new(name: &'a str) -> BlockSpec<'a> {
        BlockSpec {
            name,
            min_width: 160.0,
            weight: 1.0,
        }
    }

    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }
}

/// Greedy row packing: `(row, width)` per spec, in order.
pub(crate) fn pack(specs: &[(f32, f32)], avail: f32, gap: f32) -> Vec<(usize, f32)> {
    // specs: (min_width, weight)
    let mut out = Vec::with_capacity(specs.len());
    let mut row = 0usize;
    let mut i = 0usize;
    while i < specs.len() {
        // Greedily extend the current row while minimums fit.
        let mut end = i + 1;
        let mut min_sum = specs[i].0;
        while end < specs.len() {
            let next = min_sum + gap + specs[end].0;
            if next > avail {
                break;
            }
            min_sum = next;
            end += 1;
        }
        // Distribute the leftover by weight on top of each minimum.
        let count = end - i;
        let total_gap = gap * (count - 1) as f32;
        let mins: f32 = specs[i..end].iter().map(|s| s.0).sum();
        let leftover = (avail - total_gap - mins).max(0.0);
        let total_weight: f32 = specs[i..end].iter().map(|s| s.1.max(0.0)).sum();
        for s in &specs[i..end] {
            let share = if total_weight > 0.0 {
                leftover * s.1.max(0.0) / total_weight
            } else {
                leftover / count as f32
            };
            out.push((row, s.0 + share));
        }
        row += 1;
        i = end;
    }
    out
}

/// The layout widget. `region` is called once per spec, in order, with the
/// region's name and a Ui clipped to its slot.
pub struct BlockGrid<'a> {
    specs: &'a [BlockSpec<'a>],
    gap: f32,
}

impl<'a> BlockGrid<'a> {
    pub fn new(specs: &'a [BlockSpec<'a>]) -> BlockGrid<'a> {
        BlockGrid { specs, gap: 12.0 }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn show(self, ui: &mut Ui, mut region: impl FnMut(&str, &mut Ui)) {
        if self.specs.is_empty() {
            return;
        }
        let avail = ui.available_width();
        let mins: Vec<(f32, f32)> = self
            .specs
            .iter()
            .map(|s| (s.min_width.min(avail), s.weight))
            .collect();
        let placed = pack(&mins, avail, self.gap);

        let mut i = 0usize;
        while i < self.specs.len() {
            let row = placed[i].0;
            let end = placed[i..]
                .iter()
                .position(|(r, _)| *r != row)
                .map(|p| i + p)
                .unwrap_or(self.specs.len());

            // Lay the row's regions side by side, then advance by the
            // tallest one (same technique as egui's `Ui::columns`).
            let top_left = ui.cursor().min;
            let mut x = top_left.x;
            let mut max_h = 0.0f32;
            for (spec, (_, w)) in self.specs[i..end].iter().zip(&placed[i..end]) {
                let max_rect = Rect::from_min_max(
                    egui::pos2(x, top_left.y),
                    egui::pos2(x + w, ui.max_rect().max.y),
                );
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .max_rect(max_rect)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                child.set_width(*w);
                region(spec.name, &mut child);
                max_h = max_h.max(child.min_rect().height());
                x += w + self.gap;
            }
            ui.advance_cursor_after_rect(Rect::from_min_size(top_left, Vec2::new(avail, max_h)));
            if end < self.specs.len() {
                ui.add_space(self.gap);
            }
            i = end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_row_when_minimums_fit_widths_follow_weights() {
        // Two regions, weight 2:1, plenty of room.
        let placed = pack(&[(100.0, 2.0), (100.0, 1.0)], 400.0, 10.0);
        assert_eq!(placed[0].0, 0);
        assert_eq!(placed[1].0, 0);
        // leftover = 400 - 10 - 200 = 190 → +126.66 / +63.33
        assert!((placed[0].1 - (100.0 + 190.0 * 2.0 / 3.0)).abs() < 0.01);
        assert!((placed[1].1 - (100.0 + 190.0 / 3.0)).abs() < 0.01);
        let total: f32 = placed.iter().map(|(_, w)| w).sum::<f32>() + 10.0;
        assert!((total - 400.0).abs() < 0.01);
    }

    #[test]
    fn wraps_to_new_row_below_min_widths() {
        let placed = pack(&[(200.0, 1.0), (200.0, 1.0), (200.0, 1.0)], 450.0, 10.0);
        // Two fit (200+10+200=410 ≤ 450), third wraps.
        assert_eq!(placed[0].0, 0);
        assert_eq!(placed[1].0, 0);
        assert_eq!(placed[2].0, 1);
        // The wrapped region takes the whole row.
        assert!((placed[2].1 - 450.0).abs() < 0.01);
    }

    #[test]
    fn regions_never_shrink_below_min() {
        let placed = pack(&[(300.0, 1.0), (120.0, 5.0)], 440.0, 10.0);
        assert_eq!(placed[0].0, 0);
        assert_eq!(placed[1].0, 0);
        assert!(placed[0].1 >= 300.0);
        assert!(placed[1].1 >= 120.0);
    }
}
