//! Block chrome: the hover gutter handle ("⋮⋮") opening the block menu —
//! move/duplicate/delete, "turn into" conversions for text kinds, column
//! wrapping at the root, and add/remove column inside a columns layout.

use super::{Action, Ecx};
use crate::response::{ForgeResponse, Outcome};
use crate::widgets::overlays::{DropdownMenu, MenuItem};
use egui::{Popup, Pos2, Rect, Sense, Ui, Vec2};
use forge_blocks::{Address, BlockKind, Document, ListStyle};

/// The left gutter cell of one block row: an (invisible until hovered)
/// handle that opens the block menu.
pub(super) fn gutter(ui: &mut Ui, ecx: &mut Ecx, doc: &Document, addr: Address, row_top: f32) {
    let t = ecx.t;
    let (items, actions) = menu_entries(doc, addr);
    let hover_band = Rect::from_min_max(
        Pos2::new(ui.cursor().left() - 4.0, row_top - 2.0),
        Pos2::new(ui.max_rect().right(), row_top + 24.0),
    );
    let band_hovered = ui.rect_contains_pointer(hover_band);

    let choice = DropdownMenu::new(&items).min_width(170.0).show(ui, |ui| {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(14.0, 20.0), Sense::click());
        let open = Popup::is_id_open(ui.ctx(), Popup::default_response_id(&resp));
        if band_hovered || open || resp.hovered() {
            let color = if resp.hovered() || open {
                t.fg[1]
            } else {
                t.fg[3]
            };
            let g = ui
                .painter()
                .layout_no_wrap("⋮⋮".to_owned(), t.mono(t.type_scale.sm), color);
            ui.painter().galley(
                Pos2::new(
                    rect.center().x - g.size().x / 2.0,
                    rect.center().y - g.size().y / 2.0,
                ),
                g,
                color,
            );
        }
        ForgeResponse::new(resp, Outcome::Ignored)
    });
    if let Some(i) = choice {
        if let Some(action) = actions.get(i) {
            ecx.actions.push(action.clone());
        }
    }
}

/// The block menu rows and their matching actions, built from the block's
/// kind and position.
fn menu_entries(doc: &Document, addr: Address) -> (Vec<MenuItem>, Vec<Action>) {
    let mut items = Vec::new();
    let mut actions = Vec::new();
    let (list, idx) = super::siblings(doc, addr);
    let kind = doc.block(addr).map(|b| &b.kind);
    let md = kind.and_then(|k| k.md()).unwrap_or("").to_owned();
    let is_text = kind.is_some_and(|k| k.is_text());

    let entry =
        |items: &mut Vec<MenuItem>, actions: &mut Vec<Action>, item: MenuItem, action: Action| {
            items.push(item);
            actions.push(action);
        };

    entry(
        &mut items,
        &mut actions,
        MenuItem::new("Move up").disabled(idx == 0),
        Action::MoveBlock { addr, dir: -1 },
    );
    entry(
        &mut items,
        &mut actions,
        MenuItem::new("Move down").disabled(idx + 1 >= list.len()),
        Action::MoveBlock { addr, dir: 1 },
    );
    entry(
        &mut items,
        &mut actions,
        MenuItem::new("Duplicate"),
        Action::Duplicate(addr),
    );
    entry(
        &mut items,
        &mut actions,
        MenuItem::new("Delete").danger(true),
        Action::Remove(addr),
    );

    if is_text {
        let turn = |label: &str, kind: BlockKind| (MenuItem::new(label), kind);
        let conversions = [
            turn("Text", BlockKind::Paragraph { md: md.clone() }),
            turn(
                "Heading 1",
                BlockKind::Heading {
                    level: 1,
                    md: md.clone(),
                },
            ),
            turn(
                "Heading 2",
                BlockKind::Heading {
                    level: 2,
                    md: md.clone(),
                },
            ),
            turn(
                "Heading 3",
                BlockKind::Heading {
                    level: 3,
                    md: md.clone(),
                },
            ),
            turn(
                "Bullet list",
                BlockKind::ListItem {
                    style: ListStyle::Bullet,
                    checked: None,
                    indent: 0,
                    md: md.clone(),
                },
            ),
            turn(
                "Numbered list",
                BlockKind::ListItem {
                    style: ListStyle::Number,
                    checked: None,
                    indent: 0,
                    md: md.clone(),
                },
            ),
            turn(
                "Todo list",
                BlockKind::ListItem {
                    style: ListStyle::Todo,
                    checked: Some(false),
                    indent: 0,
                    md: md.clone(),
                },
            ),
            turn("Quote", BlockKind::Quote { md: md.clone() }),
            turn(
                "Callout",
                BlockKind::Admonition {
                    tone: forge_blocks::Tone::Info,
                    title: String::new(),
                    md: md.clone(),
                },
            ),
        ];
        for (i, (item, kind)) in conversions.into_iter().enumerate() {
            entry(
                &mut items,
                &mut actions,
                if i == 0 {
                    item.separator_before(true)
                } else {
                    item
                },
                Action::TurnInto { addr, kind },
            );
        }
    }

    match addr {
        Address::Root(_) => {
            if !matches!(kind, Some(BlockKind::Columns { .. })) {
                entry(
                    &mut items,
                    &mut actions,
                    MenuItem::new("2 columns").separator_before(true),
                    Action::WrapColumns { addr, n: 2 },
                );
                entry(
                    &mut items,
                    &mut actions,
                    MenuItem::new("3 columns"),
                    Action::WrapColumns { addr, n: 3 },
                );
            }
        }
        Address::Cell { root, col, .. } => {
            let ncols = match doc.blocks.get(root).map(|b| &b.kind) {
                Some(BlockKind::Columns { columns }) => columns.len(),
                _ => 0,
            };
            entry(
                &mut items,
                &mut actions,
                MenuItem::new("Add column")
                    .separator_before(true)
                    .disabled(ncols >= 4),
                Action::AddColumn { root },
            );
            entry(
                &mut items,
                &mut actions,
                MenuItem::new("Remove column"),
                Action::RemoveColumn { root, col },
            );
        }
    }

    (items, actions)
}
