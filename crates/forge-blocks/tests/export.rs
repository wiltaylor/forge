//! The committed JSON the block generators read must match the registry.
//!
//! `just check` regenerates the TypeScript from these files with Node alone,
//! so it cannot notice a registry edit that never reached them. This test is
//! the other half: edit the registry without running `just generate-blocks`
//! and `cargo test` says so.

use forge_blocks::export::{emoji_json, registry_json, EMOJI_PATH, ID_PLACEHOLDER, REGISTRY_PATH};

/// The committed files, read at compile time so a missing one fails loudly.
const COMMITTED_REGISTRY: &str = include_str!("../../../contract/blocks-registry.json");
const COMMITTED_EMOJI: &str = include_str!("../../../contract/emoji.json");

#[track_caller]
fn assert_current(path: &str, committed: &str, rendered: String) {
    if committed == rendered {
        return;
    }
    panic!("{path} no longer matches the registry — run `just generate-blocks` and commit it");
}

#[test]
fn the_committed_registry_is_current() {
    assert_current(REGISTRY_PATH, COMMITTED_REGISTRY, registry_json());
}

#[test]
fn the_committed_emoji_table_is_current() {
    assert_current(EMOJI_PATH, COMMITTED_EMOJI, emoji_json());
}

/// Every block id inside `value`, however deep.
fn block_ids(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let (Some(id), Some(_)) = (map.get("id").and_then(|v| v.as_str()), map.get("type")) {
                out.push(id.to_owned());
            }
            for child in map.values() {
                block_ids(child, out);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|i| block_ids(i, out)),
        _ => {}
    }
}

#[test]
fn a_committed_starter_holds_no_minted_id() {
    // A starter is a template. Committing the id `Block::new` minted while
    // rendering would make every document the generated constructor writes
    // share it.
    let dump: serde_json::Value = serde_json::from_str(&registry_json()).unwrap();
    let mut ids = Vec::new();
    for kind in dump["kinds"].as_array().unwrap() {
        block_ids(&kind["starter"], &mut ids);
    }
    for row in dump["palette"].as_array().unwrap() {
        block_ids(&row["insert"], &mut ids);
    }
    assert!(!ids.is_empty(), "no nested block found to check");
    for id in ids {
        assert_eq!(id, ID_PLACEHOLDER, "a starter committed a minted id");
    }
}
