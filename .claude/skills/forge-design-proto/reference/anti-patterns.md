# Anti-patterns

<!-- PROTOTYPE (wayfinder #64). Realistic size, not complete content. -->

Named failures. If your output matches an entry, it is wrong — whatever a control page
appeared to allow.

Entries marked **provisional** come from prior art rather than from an observed Forge
failure. They stay until a real failure confirms or replaces them.

## Look

**The accent panel.** A card, banner or sidebar filled with the accent colour. The accent
is a ring, a small solid, or text. Fix: `surface-raised` with an accent 1px border.

**The soft card.** A drop shadow, a gradient, a frosted-glass blur, or a radius above 6px.
Forge is flat and square-ish. Fix: 1px `border`, no shadow.

**The pill button.** A fully rounded button. Fix: 4px radius, 32px high.

**Unicode as icon.** `→`, `✓`, `✗`, `•` standing in for an icon in a graphical target.
Fix: a real icon at 1.5px stroke in `currentColor`. In a terminal this inverts — glyphs
*are* the icons, and an icon font is wrong.

**Emoji anywhere.** Never, on any platform.

**Colour alone.** A red dot with no word and no glyph. Fix: pair it.

## Behaviour

**The Escape avalanche.** One Escape press closes a menu and the modal behind it. Fix:
innermost only, one layer per press.

**The focus thief.** A toast, tooltip or newly mounted panel that takes focus. Only a
modal and a sheet take focus, and only on open.

**Focus lost on close.** An overlay closes and focus lands on the document body. Fix:
restore focus to whatever opened it.

**Tab inside the list.** Tab moving between the options of a select, listbox or menu. Fix:
arrows move within, Tab leaves.

**The growing row.** A table or list row that expands to fit long content, so rows are
different heights. Fix: 32px, truncate.

**The reflowed form.** A form that becomes two columns on a wide screen. Forge forms are
one column at every width.

## Accessibility

Coarse rules only. Fine ARIA detail lives here as "do not", never on a control page as
"do" — [#65](https://github.com/wiltaylor/forge/issues/65) measured detailed a11y prose as
flat-to-worse, because more rules produce more wrongly-applied attributes.

**Roles invented.** `role="combo"`, `role="dropdown"`. Only real ARIA roles exist. If you
are unsure of the role, omit it — a div with correct keyboard behaviour beats a wrong role.

**`aria-label` on everything.** Labelling something that already has a visible text label,
so screen readers announce it twice. **provisional**

**`tabindex` above 0.** Never. Only `0` and `-1` exist.

**`aria-hidden` on a focusable thing.** Hides it from readers while leaving it tabbable.

## Guidance-only failures

**Reinventing a streaming widget.** Building a terminal emulator, a VNC client or an RDP
client from these pages. They are not in Forge. Say so and stop.

**Porting by eye.** Writing an egui control by translating the SolidJS one you just wrote,
without reading the egui implementation page. The browser supplies focus, tab order,
scroll and ARIA. egui supplies none of them. **provisional**

**Naming what does not exist.** Referring to a Forge control, token role or class that is
not in `SKILL.md` or `laws.md`. A name that does not resolve is worse than no name — the
reader trusts it and ships something unstyled.
