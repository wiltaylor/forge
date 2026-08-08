//! The kind registry: one entry per schema variant. The load-bearing test is
//! [`every_schema_variant_has_an_entry`] — it enumerates the variants through
//! serde, so a new variant with no registry entry fails here rather than
//! waiting for a kit to notice.

use forge_blocks::{kind_entry, palette_rows, starter_kind, BlockKind, PaletteAction, KINDS};

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

/* ---------------- the wire shape ----------------------------------------- */

#[test]
fn declared_fields_match_what_a_starter_serializes() {
    // The generated TypeScript union is these field lists. A field the schema
    // writes and the entry does not declare would be missing from the union;
    // a required field the schema stopped writing would be a lie in it.
    //
    // A starter is minimal, so it proves the required fields and cannot prove
    // the optional ones — `registry::fields_are_exhaustive` is the compile-time
    // half that fails when a variant's field set changes at all.
    for entry in KINDS {
        let value = serde_json::to_value((entry.starter)()).unwrap();
        let object = value.as_object().expect("a kind serializes as an object");
        let declared: Vec<&str> = entry.fields.iter().map(|f| f.name).collect();

        for key in object.keys().filter(|k| *k != "type") {
            assert!(
                declared.contains(&key.as_str()),
                "`{}` serializes `{key}`, which the registry does not declare",
                entry.type_name
            );
        }
        for field in entry.fields.iter().filter(|f| !f.optional) {
            assert!(
                object.contains_key(field.name),
                "`{}` declares `{}` required, but the starter omits it",
                entry.type_name,
                field.name
            );
        }
        assert!(
            entry.fields.iter().all(|f| !f.ts.is_empty()),
            "`{}` has a field with no TypeScript type",
            entry.type_name
        );
    }
}

#[test]
fn no_kind_declares_a_field_twice() {
    for entry in KINDS {
        let mut names: Vec<&str> = entry.fields.iter().map(|f| f.name).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            names.len(),
            count,
            "`{}` declares a field twice",
            entry.type_name
        );
    }
}

/* ---------------- the slash palette --------------------------------------- */

#[test]
fn every_palette_row_makes_the_kind_it_belongs_to() {
    for entry in KINDS {
        for row in entry.palette {
            let PaletteAction::Insert(make) = row.action else {
                continue;
            };
            assert_eq!(
                make().type_name(),
                entry.type_name,
                "the palette row `{}` sits under `{}` but makes another kind",
                row.id,
                entry.type_name
            );
        }
    }
}

#[test]
fn palette_rows_are_named_once_each() {
    // A kit lists rows by label and matches them by id. Two rows sharing
    // either one would make the palette ambiguous.
    let rows = palette_rows();
    assert!(
        rows.len() > KINDS.len(),
        "a palette this short lists no variants"
    );
    for row in &rows {
        assert!(!row.label.is_empty(), "the row `{}` has no label", row.id);
        assert!(!row.id.is_empty(), "the row `{}` has no id", row.label);
    }
    for key in [
        rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        rows.iter().map(|r| r.label).collect::<Vec<_>>(),
    ] {
        let mut sorted = key.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            key.len(),
            "two palette rows share a key: {key:?}"
        );
    }
}

#[test]
fn the_wrap_actions_come_last() {
    // Every kit lists the column commands at the end of the palette. They are
    // commands rather than kinds, so `palette_rows` moves them there whatever
    // order the registry declares its kinds in.
    let rows = palette_rows();
    let first_wrap = rows
        .iter()
        .position(|r| matches!(r.action, PaletteAction::WrapColumns(_)))
        .expect("the palette offers columns");
    assert!(
        rows[first_wrap..]
            .iter()
            .all(|r| matches!(r.action, PaletteAction::WrapColumns(_))),
        "an insert row sits after a wrap action"
    );
    assert_eq!(
        rows[first_wrap..].iter().map(|r| r.id).collect::<Vec<_>>(),
        ["col2", "col3"]
    );
}

#[test]
fn every_kind_but_custom_reaches_the_palette() {
    // `custom` is the exception: a kit lists one row per kind the host
    // registered, so the registry cannot name them.
    for entry in KINDS {
        assert_eq!(
            entry.palette.is_empty(),
            entry.type_name == "custom",
            "`{}` and the palette disagree",
            entry.type_name
        );
    }
}

#[test]
fn the_first_row_of_a_kind_inserts_its_starter() {
    // The palette is where a starter divergence used to show up: one kit's
    // table was two by one while the others' was three by two. A kind that
    // offers several rows offers variants of one starter — the first heading
    // level, the first list style — so the first row is the starter itself,
    // whatever else follows it.
    for entry in KINDS {
        let Some(row) = entry.palette.first() else {
            continue;
        };
        let PaletteAction::Insert(make) = row.action else {
            continue;
        };
        assert_eq!(
            make(),
            (entry.starter)(),
            "the palette row `{}` inserts something other than the `{}` starter",
            row.id,
            entry.type_name
        );
    }
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

#[test]
fn the_palette_offers_heading_levels_one_to_four() {
    // The other shape a divergence took: one kit offered three heading levels
    // while the others offered four. Every kit reads these rows, so the levels
    // it offers are the levels named here.
    let levels: Vec<u8> = kind_entry("heading")
        .unwrap()
        .palette
        .iter()
        .map(|row| {
            let PaletteAction::Insert(make) = row.action else {
                panic!("the heading row `{}` does not insert", row.id);
            };
            match make() {
                BlockKind::Heading { level, .. } => level,
                other => panic!("the heading row `{}` makes {other:?}", row.id),
            }
        })
        .collect();
    assert_eq!(levels, [1, 2, 3, 4]);
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
