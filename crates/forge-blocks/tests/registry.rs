//! The kind registry: one entry per schema variant. The load-bearing test is
//! [`every_schema_variant_has_an_entry`] — it enumerates the variants through
//! serde, so a new variant with no registry entry fails here rather than
//! waiting for a kit to notice.

use forge_blocks::{kind_entry, starter_kind, BlockKind, KINDS};

/// Every wire `type` name the schema accepts, taken from serde's own list of
/// expected variants (the error text for an unknown tag). No hand-kept copy.
fn schema_type_names() -> Vec<String> {
    let err = serde_json::from_value::<BlockKind>(serde_json::json!({ "type": "__unknown__" }))
        .expect_err("an unknown tag must not deserialize")
        .to_string();
    let (_, list) = err
        .split_once("expected one of ")
        .expect("serde lists the expected variants");
    list.split(", ")
        .map(|name| name.trim_matches('`').to_string())
        .collect()
}

#[test]
fn every_schema_variant_has_an_entry() {
    let names = schema_type_names();
    assert!(names.len() > 1, "variant list looks unparsed: {names:?}");
    for name in &names {
        assert!(
            kind_entry(name).is_some(),
            "schema variant `{name}` has no registry entry"
        );
    }
    let registered: Vec<&str> = KINDS.iter().map(|e| e.type_name).collect();
    assert_eq!(
        registered, names,
        "the registry and the schema list different kinds"
    );
}

#[test]
fn entries_are_reachable_and_distinct() {
    for entry in KINDS {
        let found = kind_entry(entry.type_name).expect(entry.type_name);
        assert!(
            std::ptr::eq(found, entry),
            "`{}` resolves to another entry",
            entry.type_name
        );
        assert!(
            !entry.label.is_empty(),
            "`{}` has no label",
            entry.type_name
        );
    }
    let mut labels: Vec<&str> = KINDS.iter().map(|e| e.label).collect();
    labels.sort_unstable();
    let count = labels.len();
    labels.dedup();
    assert_eq!(labels.len(), count, "two kinds share a label");
    assert!(kind_entry("nope").is_none());
}

#[test]
fn starters_carry_their_own_kind() {
    for entry in KINDS {
        let kind = (entry.starter)();
        assert_eq!(
            kind.type_name(),
            entry.type_name,
            "the starter for `{}` is another kind",
            entry.type_name
        );
        assert_eq!(
            serde_json::to_value(&kind).unwrap()["type"],
            serde_json::json!(entry.type_name),
            "the starter for `{}` serializes under another tag",
            entry.type_name
        );
        assert_eq!(kind.entry().type_name, entry.type_name);
    }
}

#[test]
fn data_ness_matches_the_predicate() {
    // The registry records data-ness; `is_data` keeps routing on it. They must
    // not drift.
    for entry in KINDS {
        assert_eq!(
            entry.is_data,
            (entry.starter)().is_data(),
            "the registry and `is_data` disagree about `{}`",
            entry.type_name
        );
    }
}

#[test]
fn starter_kind_serves_the_shared_starters() {
    // `starter_kind` reads the registry now. The names it answers to are the
    // ones it answered to before, written out rather than derived, so that a
    // change to the rule behind them fails here.
    const SHARED: &[&str] = &[
        "image",
        "video",
        "math",
        "bar_chart",
        "line_chart",
        "pie_chart",
        "diagram",
        "sequence_diagram",
        "state_diagram",
        "node_table",
        "tree",
        "timeline",
        "chapter_header",
        "footnote",
    ];
    for entry in KINDS {
        let shared = starter_kind(entry.type_name);
        assert_eq!(
            shared.is_some(),
            SHARED.contains(&entry.type_name),
            "`starter_kind` changed its mind about `{}`",
            entry.type_name
        );
        if let Some(kind) = shared {
            assert_eq!(
                kind,
                (entry.starter)(),
                "two starters for `{}`",
                entry.type_name
            );
        }
    }
    assert!(starter_kind("nope").is_none());
}

#[test]
fn table_starter_is_three_by_two() {
    // The majority shape: two kits already start a table three wide and two
    // rows deep.
    let BlockKind::Table { header, rows } = (kind_entry("table").unwrap().starter)() else {
        panic!("the table starter is not a table");
    };
    assert_eq!(header.len(), 3);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.len() == 3));
}

/// The markdown one starter block writes, and the kinds it reads back as.
#[cfg(feature = "md")]
fn round_trip(kind: BlockKind) -> (String, Vec<BlockKind>) {
    use forge_blocks::{from_markdown, to_markdown, Block, Document};

    let md = to_markdown(&Document::from_blocks(vec![Block::new(kind)]));
    let back = from_markdown(&md)
        .blocks
        .into_iter()
        .map(|b| b.kind)
        .collect();
    (md, back)
}

#[cfg(feature = "md")]
#[test]
fn markdown_form_matches_the_converter() {
    use forge_blocks::MarkdownForm;

    for entry in KINDS {
        let starter = (entry.starter)();
        let (md, back) = round_trip(starter.clone());
        let name = entry.type_name;
        // Every form but the flattened one survives the trip out and back.
        match entry.markdown {
            MarkdownForm::Flattened => {
                assert_ne!(
                    back,
                    vec![starter],
                    "`{name}` claims to flatten but did not"
                );
            }
            _ => assert_eq!(back, vec![starter], "`{name}` did not survive `{md}`"),
        }
        // And the fences say which fence they are.
        match entry.markdown {
            MarkdownForm::Native | MarkdownForm::NativeOrFence => assert!(
                !md.starts_with("```forge:"),
                "`{name}` claims markdown's own syntax but writes a fence"
            ),
            MarkdownForm::Fence => assert!(
                md.starts_with(&format!("```forge:{name}")),
                "`{name}` claims a fence but writes `{md}`"
            ),
            MarkdownForm::CustomFence => assert!(
                md.starts_with("```block:"),
                "`{name}` claims a custom fence but writes `{md}`"
            ),
            MarkdownForm::Flattened => assert!(
                !md.starts_with("```"),
                "`{name}` claims to flatten but writes a fence"
            ),
        }
    }
}

#[cfg(feature = "md")]
#[test]
fn a_sized_image_falls_back_to_a_fence() {
    // What `NativeOrFence` means: markdown's own syntax where the fields allow
    // it, and a fence where they do not.
    use forge_blocks::MarkdownForm;

    let sized = BlockKind::Image {
        src: "a.png".into(),
        alt: String::new(),
        width: Some(320.0),
        height: None,
    };
    assert_eq!(sized.entry().markdown, MarkdownForm::NativeOrFence);
    let (md, back) = round_trip(sized.clone());
    assert!(md.starts_with("```forge:image"), "got `{md}`");
    assert_eq!(back, vec![sized]);
}

#[cfg(feature = "md")]
#[test]
fn a_multi_line_footnote_falls_back_to_a_fence() {
    let long = BlockKind::Footnote {
        label: "note-1".into(),
        md: "one\ntwo".into(),
    };
    let (md, back) = round_trip(long.clone());
    assert!(md.starts_with("```forge:footnote"), "got `{md}`");
    assert_eq!(back, vec![long]);
}
