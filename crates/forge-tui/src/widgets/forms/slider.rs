use crate::event::{in_area, is_press, left_down, Outcome};
use crate::text;
use crate::theme::{default_theme, Severity, Theme};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::StatefulWidget;

#[derive(Clone, Copy, Debug)]
pub struct SliderState {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    track: Rect,
}

impl SliderState {
    pub fn new(value: f64, min: f64, max: f64, step: f64) -> SliderState {
        SliderState {
            value: value.clamp(min, max),
            min,
            max,
            step,
            track: Rect::default(),
        }
    }

    /// Click or drag along the track to set the value (snapped to `step`).
    pub fn handle_mouse(&mut self, ev: &MouseEvent) -> Outcome {
        let dragging = matches!(ev.kind, MouseEventKind::Drag(_));
        if !(left_down(ev) || dragging) || !in_area(ev, self.track) || self.track.width < 2 {
            return Outcome::Ignored;
        }
        let ratio = (ev.column - self.track.x) as f64 / (self.track.width - 1) as f64;
        let raw = self.min + ratio * (self.max - self.min);
        let snapped = if self.step > 0.0 {
            (((raw - self.min) / self.step).round() * self.step + self.min)
                .clamp(self.min, self.max)
        } else {
            raw.clamp(self.min, self.max)
        };
        if (snapped - self.value).abs() > f64::EPSILON {
            self.value = snapped;
            Outcome::Changed
        } else {
            Outcome::Consumed
        }
    }

    pub fn ratio(&self) -> f64 {
        if self.max <= self.min {
            0.0
        } else {
            (self.value - self.min) / (self.max - self.min)
        }
    }

    /// ←/→ step; Shift steps ×10; Home/End jump to the bounds.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if !is_press(&key) {
            return Outcome::Ignored;
        }
        let step = if key.modifiers.contains(KeyModifiers::SHIFT) {
            self.step * 10.0
        } else {
            self.step
        };
        let target = match key.code {
            KeyCode::Left | KeyCode::Down => self.value - step,
            KeyCode::Right | KeyCode::Up => self.value + step,
            KeyCode::Home => self.min,
            KeyCode::End => self.max,
            _ => return Outcome::Ignored,
        };
        let target = target.clamp(self.min, self.max);
        if (target - self.value).abs() > f64::EPSILON {
            self.value = target;
            Outcome::Changed
        } else {
            Outcome::Consumed
        }
    }
}

/// `──────●────  42` — filled track in accent (or a semantic tone), value
/// readout at the right.
#[derive(Clone, Debug, Default)]
pub struct Slider<'a> {
    label: Option<&'a str>,
    severity: Option<Severity>,
    focused: bool,
    disabled: bool,
    show_value: bool,
    theme: Option<&'a Theme>,
}

impl<'a> Slider<'a> {
    pub fn new() -> Slider<'a> {
        Slider {
            show_value: true,
            ..Default::default()
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = Some(theme);
        self
    }
}

impl<'a> StatefulWidget for Slider<'a> {
    type State = SliderState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut SliderState) {
        if area.is_empty() {
            return;
        }
        let t = self.theme.unwrap_or_else(|| default_theme());
        let fill = if self.disabled {
            t.fg[3]
        } else {
            match self.severity {
                Some(s) => t.severity(s).base,
                None => t.accent.base,
            }
        };
        let mut x = area.x;
        let mut w = area.width;
        if let Some(label) = self.label {
            let label = text::truncate(label, (w / 3) as usize);
            let lw = text::width(&label) as u16 + 1;
            buf.set_string(x, area.y, label, Style::new().fg(t.fg[1]));
            x += lw;
            w = w.saturating_sub(lw);
        }
        let mut val_w = 0;
        let val = format!("{:.0}", state.value);
        if self.show_value {
            val_w = val.len() as u16 + 2;
            w = w.saturating_sub(val_w);
        }
        state.track = Rect::new(x, area.y, w, 1);
        if w >= 3 {
            let knob = ((w - 1) as f64 * state.ratio()).round() as u16;
            for dx in 0..w {
                let (ch, color) = if dx == knob {
                    ("●", if self.focused { t.accent.hover } else { fill })
                } else if dx < knob {
                    ("─", fill)
                } else {
                    ("─", t.border.strong)
                };
                buf.set_string(x + dx, area.y, ch, Style::new().fg(color));
            }
        }
        if self.show_value && val_w > 0 {
            buf.set_string(x + w + 2, area.y, val, Style::new().fg(t.fg[1]));
        }
    }
}
