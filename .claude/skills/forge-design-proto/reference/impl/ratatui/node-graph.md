# node-graph — ratatui

<!-- RECAST against the #66 template. A gap page's body IS the notice — it asserts no
     status field, so it cannot drift from gaps.md. #64 settled zero-hop direct
     addressing, so a reader lands here without passing an index. -->

control page: [node-graph](../../controls/node-graph.md)

**GAP — not built on ratatui.** Nearest reference: The egui page, which is the closest by far: both paint the graph by
hand and own their own hit-testing.

Fill it by building this control in a real target app from the control page, then write
this page from that working code. Do not write it from another platform's implementation
page — see "porting by eye" in [anti-patterns.md](../../anti-patterns.md).
