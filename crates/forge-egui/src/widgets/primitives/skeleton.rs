//! Shimmer loading placeholder. The moving highlight band is a vertex-colored
//! mesh; animation requests its own repaints.

use crate::theme::Theme;
use egui::epaint::{Mesh, Vertex};
use egui::{Color32, CornerRadius, Sense, Ui, Vec2};

pub struct Skeleton {
    width: Option<f32>,
    height: f32,
}

impl Skeleton {
    pub fn new() -> Skeleton {
        Skeleton {
            width: None,
            height: 14.0,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::of(ui.ctx());
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, self.height), Sense::hover());

        if ui.is_rect_visible(rect) {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(t.radius.sm as u8), t.bg[2]);

            // Moving highlight band: 1.2s cycle, band 40% of width.
            let cycle = 1.2;
            let phase = (ui.input(|i| i.time) % cycle / cycle) as f32;
            let band_w = rect.width() * 0.4;
            let x0 = rect.min.x - band_w + phase * (rect.width() + band_w);
            let highlight = crate::theme::color::with_alpha(t.bg[4], 160);
            let clear = Color32::TRANSPARENT;

            let mut mesh = Mesh::default();
            let y0 = rect.min.y;
            let y1 = rect.max.y;
            let xs = [x0, x0 + band_w / 2.0, x0 + band_w];
            let colors = [clear, highlight, clear];
            let base = mesh.vertices.len() as u32;
            for (x, color) in xs.iter().zip(colors) {
                let x = x.clamp(rect.min.x, rect.max.x);
                mesh.vertices.push(Vertex {
                    pos: egui::pos2(x, y0),
                    uv: egui::epaint::WHITE_UV,
                    color,
                });
                mesh.vertices.push(Vertex {
                    pos: egui::pos2(x, y1),
                    uv: egui::epaint::WHITE_UV,
                    color,
                });
            }
            for i in 0..2u32 {
                let o = base + i * 2;
                mesh.indices
                    .extend_from_slice(&[o, o + 1, o + 2, o + 1, o + 3, o + 2]);
            }
            ui.painter().add(mesh);
            ui.ctx().request_repaint();
        }
        response
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Skeleton::new()
    }
}
