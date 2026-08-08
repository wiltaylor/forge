//! `${name}` substitution and the matcher the expectations are written in.
//!
//! Objects match by subset, so a case asserts the fields it is about and stays
//! quiet about the rest; `$exact` opts back into deep equality. See
//! `contract/README.md` for the full table.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

/// The `${name}` substitution table.
pub type Vars = BTreeMap<String, String>;

/// Replace every `${name}` in `input`. An unknown name is an error, not an
/// empty string — a typo in the corpus should fail loudly.
pub fn interpolate(input: &str, vars: &Vars) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let tail = &rest[start + 2..];
        let Some(end) = tail.find('}') else {
            return Err(format!("unterminated ${{ in {input:?}"));
        };
        let name = &tail[..end];
        match vars.get(name) {
            Some(value) => out.push_str(value),
            None => return Err(format!("unknown variable ${{{name}}} in {input:?}")),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Interpolate every string in a JSON value, keys included.
pub fn interpolate_value(value: &Value, vars: &Vars) -> Result<Value, String> {
    Ok(match value {
        Value::String(s) => Value::String(interpolate(s, vars)?),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| interpolate_value(v, vars))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(map) => {
            let mut out = Map::with_capacity(map.len());
            for (k, v) in map {
                out.insert(interpolate(k, vars)?, interpolate_value(v, vars)?);
            }
            Value::Object(out)
        }
        other => other.clone(),
    })
}

/// Check `actual` against the authored matcher `expected`.
///
/// The error names the path it failed at, so a mismatch deep in an envelope
/// points at the field rather than at the whole body.
pub fn match_value(expected: &Value, actual: &Value, vars: &Vars) -> Result<(), String> {
    check(expected, actual, vars, "$")
}

fn check(expected: &Value, actual: &Value, vars: &Vars, path: &str) -> Result<(), String> {
    match expected {
        Value::Object(map) => match operator(map, path)? {
            Some((op, operand)) => check_operator(op, operand, actual, vars, path),
            None => check_subset(map, actual, vars, path),
        },
        Value::Array(items) => check_array(items, actual, vars, path),
        Value::String(s) => {
            let wanted = Value::String(interpolate(s, vars)?);
            equal(&wanted, actual, path)
        }
        other => equal(other, actual, path),
    }
}

/// An operator object holds exactly one `$` key and nothing else. A mix of the
/// two is a corpus bug that would otherwise pass silently.
fn operator<'a>(
    map: &'a Map<String, Value>,
    path: &str,
) -> Result<Option<(&'a str, &'a Value)>, String> {
    let dollars = map.keys().filter(|k| k.starts_with('$')).count();
    if dollars == 0 {
        return Ok(None);
    }
    if dollars != map.len() || map.len() != 1 {
        return Err(format!(
            "at {path}: an operator object holds exactly one $ key, found {:?}",
            map.keys().collect::<Vec<_>>()
        ));
    }
    let (key, operand) = map.iter().next().expect("one entry");
    Ok(Some((key.as_str(), operand)))
}

fn check_operator(
    op: &str,
    operand: &Value,
    actual: &Value,
    vars: &Vars,
    path: &str,
) -> Result<(), String> {
    match op {
        "$exact" => check_exact(operand, actual, vars, path),
        "$min_length" => {
            let wanted = operand
                .as_u64()
                .ok_or_else(|| format!("at {path}: $min_length takes a count"))?
                as usize;
            match actual.as_str() {
                Some(s) if s.chars().count() >= wanted => Ok(()),
                Some(s) => Err(format!(
                    "at {path}: {s:?} is shorter than {wanted} characters"
                )),
                None => Err(format!(
                    "at {path}: $min_length needs a string, got {actual}"
                )),
            }
        }
        "$type" => {
            let wanted = operand
                .as_str()
                .ok_or_else(|| format!("at {path}: $type takes a type name"))?;
            let ok = match wanted {
                "string" => actual.is_string(),
                "number" => actual.is_number(),
                "integer" => actual.is_i64() || actual.is_u64(),
                "boolean" => actual.is_boolean(),
                "array" => actual.is_array(),
                "object" => actual.is_object(),
                "null" => actual.is_null(),
                other => return Err(format!("at {path}: unknown $type {other:?}")),
            };
            if ok {
                Ok(())
            } else {
                Err(format!("at {path}: expected a {wanted}, got {actual}"))
            }
        }
        "$contains" => match actual {
            Value::String(haystack) => {
                let needle = operand
                    .as_str()
                    .ok_or_else(|| format!("at {path}: $contains on a string takes a string"))?;
                let needle = interpolate(needle, vars)?;
                if haystack.contains(&needle) {
                    Ok(())
                } else {
                    Err(format!(
                        "at {path}: {haystack:?} does not contain {needle:?}"
                    ))
                }
            }
            Value::Array(items) => {
                if items
                    .iter()
                    .any(|item| check(operand, item, vars, path).is_ok())
                {
                    Ok(())
                } else {
                    Err(format!(
                        "at {path}: no element matches {operand}, got {actual}"
                    ))
                }
            }
            other => Err(format!(
                "at {path}: $contains needs a string or an array, got {other}"
            )),
        },
        "$prefix" => {
            let wanted = operand
                .as_str()
                .ok_or_else(|| format!("at {path}: $prefix takes a string"))?;
            let wanted = interpolate(wanted, vars)?;
            match actual.as_str() {
                Some(s) if s.starts_with(&wanted) => Ok(()),
                Some(s) => Err(format!("at {path}: {s:?} does not start with {wanted:?}")),
                None => Err(format!("at {path}: $prefix needs a string, got {actual}")),
            }
        }
        "$gt" => {
            let wanted = operand
                .as_f64()
                .ok_or_else(|| format!("at {path}: $gt takes a number"))?;
            match actual.as_f64() {
                Some(n) if n > wanted => Ok(()),
                Some(n) => Err(format!("at {path}: {n} is not greater than {wanted}")),
                None => Err(format!("at {path}: $gt needs a number, got {actual}")),
            }
        }
        other => Err(format!("at {path}: unknown matcher {other:?}")),
    }
}

/// `$exact`: no extra keys, anywhere below this point. Matchers still apply,
/// so a payload whose shape is the point can still say `{"$type": "integer"}`
/// for the one field that moves.
fn check_exact(expected: &Value, actual: &Value, vars: &Vars, path: &str) -> Result<(), String> {
    match expected {
        Value::Object(map) => {
            if let Some((op, operand)) = operator(map, path)? {
                return check_operator(op, operand, actual, vars, path);
            }
            let Some(got) = actual.as_object() else {
                return Err(format!("at {path}: expected an object, got {actual}"));
            };
            let mut wanted = BTreeMap::new();
            for (key, want) in map {
                wanted.insert(interpolate(key, vars)?, want);
            }
            for key in got.keys() {
                if !wanted.contains_key(key) {
                    return Err(format!("at {path}.{key}: unexpected"));
                }
            }
            for (key, want) in wanted {
                let child = format!("{path}.{key}");
                let Some(got) = got.get(&key) else {
                    return Err(format!("at {child}: missing"));
                };
                check_exact(want, got, vars, &child)?;
            }
            Ok(())
        }
        Value::Array(items) => {
            let Some(got) = actual.as_array() else {
                return Err(format!("at {path}: expected an array, got {actual}"));
            };
            if items.len() != got.len() {
                return Err(format!(
                    "at {path}: expected {} elements, got {}",
                    items.len(),
                    got.len()
                ));
            }
            for (i, (want, got)) in items.iter().zip(got).enumerate() {
                check_exact(want, got, vars, &format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        Value::String(s) => equal(&Value::String(interpolate(s, vars)?), actual, path),
        other => equal(other, actual, path),
    }
}

fn check_subset(
    expected: &Map<String, Value>,
    actual: &Value,
    vars: &Vars,
    path: &str,
) -> Result<(), String> {
    let Some(actual) = actual.as_object() else {
        return Err(format!("at {path}: expected an object, got {actual}"));
    };
    for (key, want) in expected {
        let key = interpolate(key, vars)?;
        let child = format!("{path}.{key}");
        let Some(got) = actual.get(&key) else {
            return Err(format!("at {child}: missing"));
        };
        check(want, got, vars, &child)?;
    }
    Ok(())
}

fn check_array(expected: &[Value], actual: &Value, vars: &Vars, path: &str) -> Result<(), String> {
    let Some(actual) = actual.as_array() else {
        return Err(format!("at {path}: expected an array, got {actual}"));
    };
    if expected.len() != actual.len() {
        return Err(format!(
            "at {path}: expected {} elements, got {}",
            expected.len(),
            actual.len()
        ));
    }
    for (i, (want, got)) in expected.iter().zip(actual).enumerate() {
        check(want, got, vars, &format!("{path}[{i}]"))?;
    }
    Ok(())
}

fn equal(expected: &Value, actual: &Value, path: &str) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(format!("at {path}: expected {expected}, got {actual}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vars() -> Vars {
        Vars::from([("user".to_string(), "admin".to_string())])
    }

    fn ok(expected: Value, actual: Value) {
        match_value(&expected, &actual, &vars()).expect("should match");
    }

    fn fails(expected: Value, actual: Value) -> String {
        match_value(&expected, &actual, &vars()).expect_err("should not match")
    }

    #[test]
    fn objects_match_by_subset() {
        ok(json!({"ok": true}), json!({"ok": true, "data": 1}));
        assert!(fails(json!({"ok": true}), json!({"data": 1})).contains("$.ok: missing"));
    }

    #[test]
    fn exact_rejects_extra_keys() {
        ok(json!({"$exact": {"n": 1}}), json!({"n": 1}));
        assert!(
            fails(json!({"$exact": {"n": 1}}), json!({"n": 1, "x": 2})).contains("$.x: unexpected")
        );
        assert!(fails(json!({"$exact": {"n": 1}}), json!({})).contains("$.n: missing"));
        // Nested too: exactness reaches all the way down.
        assert!(fails(
            json!({"$exact": {"a": {"b": 1}}}),
            json!({"a": {"b": 1, "c": 2}})
        )
        .contains("$.a.c: unexpected"));
    }

    /// The claims payload is a fixed key set with one field that moves, so
    /// `$exact` has to hold a matcher rather than only literals.
    #[test]
    fn exact_still_runs_the_matchers_inside_it() {
        ok(
            json!({"$exact": {"sub": "${user}", "exp": {"$type": "integer"}}}),
            json!({"sub": "admin", "exp": 1234}),
        );
        let err = fails(
            json!({"$exact": {"sub": "${user}", "exp": {"$type": "integer"}}}),
            json!({"sub": "admin", "exp": "soon"}),
        );
        assert!(err.contains("at $.exp: expected a integer"), "{err}");
    }

    #[test]
    fn arrays_match_element_wise_and_by_length() {
        ok(json!([{"a": 1}]), json!([{"a": 1, "b": 2}]));
        assert!(fails(json!([1]), json!([1, 2])).contains("expected 1 elements, got 2"));
    }

    #[test]
    fn contains_reads_strings_and_arrays() {
        ok(json!({"$contains": "echo"}), json!(["echo", "publish"]));
        ok(json!({"$contains": "echo"}), json!("no action echo here"));
        ok(
            json!({"$contains": {"name": "b"}}),
            json!([{"name": "a"}, {"name": "b"}]),
        );
        assert!(fails(json!({"$contains": "gone"}), json!(["echo"])).contains("no element"));
    }

    #[test]
    fn type_and_number_matchers() {
        ok(json!({"$type": "integer"}), json!(3));
        ok(json!({"$gt": 0}), json!(1.5));
        assert!(fails(json!({"$type": "integer"}), json!(1.5)).contains("expected a integer"));
        assert!(fails(json!({"$gt": 0}), json!(0)).contains("not greater"));
    }

    /// `$type: "string"` would accept `""`, which is what a token must never be.
    #[test]
    fn min_length_rejects_the_empty_string() {
        ok(json!({"$min_length": 1}), json!("a"));
        assert!(fails(json!({"$min_length": 1}), json!("")).contains("shorter than 1"));
        assert!(fails(json!({"$min_length": 1}), json!(7)).contains("needs a string"));
    }

    #[test]
    fn strings_interpolate_before_comparing() {
        ok(json!("${user}"), json!("admin"));
        assert!(fails(json!("${user}"), json!("ops")).contains("expected \"admin\""));
        let err = match_value(&json!("${nope}"), &json!("x"), &vars()).unwrap_err();
        assert!(err.contains("unknown variable"), "{err}");
    }

    #[test]
    fn a_mixed_operator_object_is_a_corpus_bug() {
        let err = fails(json!({"$type": "object", "n": 1}), json!({"n": 1}));
        assert!(err.contains("exactly one $ key"), "{err}");
    }

    #[test]
    fn interpolation_reaches_keys_and_nested_values() {
        let out = interpolate_value(&json!({"${user}": ["${user}", 1]}), &vars()).unwrap();
        assert_eq!(out, json!({"admin": ["admin", 1]}));
    }

    #[test]
    fn the_path_in_the_error_points_at_the_field() {
        let err = fails(
            json!({"data": {"sub": "${user}"}}),
            json!({"data": {"sub": "ops"}}),
        );
        assert!(err.starts_with("at $.data.sub:"), "{err}");
    }
}
