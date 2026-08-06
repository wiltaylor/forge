# code-editor — egui

<!-- RECAST against the #66 template. A gap page's body IS the notice — it asserts no
     status field, so it cannot drift from gaps.md. #64 settled zero-hop direct
     addressing, so a reader lands here without passing an index. -->

control page: [code-editor](../../controls/code-editor.md)

**GAP — not built on egui.** Nearest reference: The SolidJS page, which wraps CodeMirror 6. Nothing in the Rust
crates is close — there is no text-editing core to build on.

Fill it by building this control in a real target app from the control page, then write
this page from that working code. Do not write it from another platform's implementation
page — see "porting by eye" in [anti-patterns.md](../../anti-patterns.md).
