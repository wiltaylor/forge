use forge_tui::prelude::*;
use ratatui::layout::Rect;
use ratatui::Frame;

pub fn draw(frame: &mut Frame, area: Rect, ctx: &mut Ctx, t: &Theme) {
    let mut y = area.y;
    let x = area.x;
    let w = area.width;
    let bottom = area.y + area.height;
    let row = |h: u16, y: &mut u16| -> Option<Rect> {
        if *y + h > bottom {
            return None;
        }
        let r = Rect::new(x, *y, w, h);
        *y += h + 1;
        Some(r)
    };

    if let Some(r) = row(1, &mut y) {
        frame.render_widget(Eyebrow::new("Buttons").theme(t), r);
    }
    if let Some(r) = row(1, &mut y) {
        let labels: [(&str, Variant, bool); 5] = [
            ("Primary", Variant::Primary, false),
            ("Default", Variant::Default, false),
            ("Ghost", Variant::Ghost, false),
            ("Danger", Variant::Danger, false),
            ("Disabled", Variant::Default, true),
        ];
        let mut bx = r.x;
        for (i, (label, variant, disabled)) in labels.iter().enumerate() {
            let b = Button::new(label)
                .variant(*variant)
                .disabled(*disabled)
                .theme(t);
            let bw = b.width();
            if bx + bw > r.x + r.width {
                break;
            }
            let focused = !disabled && ctx.focus.register(FocusId::indexed("prim-btn", i as u32));
            frame.render_widget(b.focused(focused), Rect::new(bx, r.y, bw, 1));
            bx += bw + 2;
        }
    }

    if let Some(r) = row(1, &mut y) {
        frame.render_widget(Eyebrow::new("Badges · Status · Kbd").theme(t), r);
    }
    if let Some(r) = row(1, &mut y) {
        let mut bx = r.x;
        let badges = [
            (None, "neutral"),
            (Some(Severity::Success), "ready"),
            (Some(Severity::Warning), "degraded"),
            (Some(Severity::Danger), "down"),
            (Some(Severity::Info), "beta"),
        ];
        for (sev, label) in badges {
            let mut badge = Badge::new(label).theme(t);
            if let Some(s) = sev {
                badge = badge.severity(s);
            }
            let bw = badge.width();
            if bx + bw > r.x + r.width {
                break;
            }
            frame.render_widget(badge, Rect::new(bx, r.y, bw, 1));
            bx += bw + 1;
        }
        let dots = [
            (Severity::Success, "live", true),
            (Severity::Warning, "sync", false),
            (Severity::Danger, "err", false),
        ];
        for (sev, label, pulse) in dots {
            let need = 3 + label.len() as u16 + 2;
            if bx + need > r.x + r.width {
                break;
            }
            frame.render_widget(
                StatusDot::new(sev)
                    .label(label)
                    .pulse(pulse)
                    .frame(ctx.frame)
                    .theme(t),
                Rect::new(bx + 1, r.y, need, 1),
            );
            bx += need + 1;
        }
        if bx + 8 <= r.x + r.width {
            frame.render_widget(Kbd::new("⌃K").theme(t), Rect::new(bx + 1, r.y, 6, 1));
        }
    }

    if let Some(r) = row(1, &mut y) {
        frame.render_widget(Eyebrow::new("Stats").theme(t), r);
    }
    if let Some(r) = row(3, &mut y) {
        let stats: [(&str, &str, &str, Trend); 3] = [
            ("Requests", "48.2k", "12% vs last hour", Trend::Up),
            ("Errors", "0.4%", "0.1% vs last hour", Trend::Down),
            ("P99", "182ms", "flat", Trend::Flat),
        ];
        let cells = Grid::new(3).gap(2).cells(r, stats.len(), 3);
        for (cell, (label, value, delta, trend)) in cells.into_iter().zip(stats) {
            frame.render_widget(
                Stat::new(label, value)
                    .delta(delta, trend)
                    .up_is_good(label != "Errors")
                    .theme(t),
                cell,
            );
        }
    }

    if let Some(r) = row(1, &mut y) {
        frame.render_widget(Separator::horizontal().theme(t), r);
    }

    if let Some(r) = row(6, &mut y) {
        let cells = Grid::new(3).gap(2).cells(r, 3, 6);
        if let Some(c) = cells.first() {
            let card = Card::new().title(" Card ").footer(" footer ").theme(t);
            let inner = card.inner(*c);
            frame.render_widget(card, *c);
            frame.render_widget(
                Avatar::new("Wil Taylor").theme(t),
                Rect::new(inner.x, inner.y, inner.width, 1),
            );
        }
        if let Some(c) = cells.get(1) {
            frame.render_widget(Skeleton::new().frame(ctx.frame).theme(t), *c);
        }
        if let Some(c) = cells.get(2) {
            frame.render_widget(
                Empty::new("No results").hint("Adjust the filters").theme(t),
                *c,
            );
        }
    }
}
