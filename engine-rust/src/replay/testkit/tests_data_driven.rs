//! Shared harness for data-driven replay/solver engine tests.
//!
//! Migrated modules keep one typed row table plus a runner function holding the
//! verbatim original assertions, and expand each row into an individually named
//! `#[test]` through [`data_driven_cases!`]. Tables stay in typed Rust on
//! purpose: the runners drive crate-private [`ReplayState`](super::ReplayState)
//! internals, so serialized case data would need an ad hoc state-patch
//! interpreter, which is explicitly out of scope. The machine-readable
//! old-test-to-new-case mapping lives at
//! `engine-rust/tests/data-driven/mapping.json` and is validated by the loader
//! below, not executed as test input.

use std::collections::BTreeSet;

/// Generate one `#[test]` per data row plus a table validity test.
///
/// ```ignore
/// crate::data_driven_cases! {
///     table: MY_CASES,
///     run: run_my_case,
///     suite: "replay::tests_example::my_loop_test",
///     valid: my_loop_test_case_table_is_valid,
///     cases: [
///         my_loop_test__row_a => 0,
///         my_loop_test__row_b => 1,
///     ]
/// }
/// ```
///
/// Row indices are checked at runtime by the generated validity test: an
/// out-of-range index, a duplicated index, or a table row no case executes
/// fails that test (fail closed) instead of silently running the wrong row
/// or omitting a row.
#[macro_export]
macro_rules! data_driven_cases {
    (
        table: $table:expr,
        run: $run:path,
        suite: $suite:literal,
        valid: $valid:ident,
        cases: [$($name:ident => $row:expr),* $(,)?]
    ) => {
        $(
            #[test]
            fn $name() {
                $run(&$table[$row]);
            }
        )*
        #[test]
        fn $valid() {
            $crate::replay::tests_data_driven::validate_case_table(
                $suite,
                &[$(stringify!($name)),*],
                &[$(($row) as usize),*],
                $table.len(),
            )
            .expect("data-driven case table validates");
        }
    };
}

/// Fail-closed validator for generated case tables.
///
/// Rejects empty suites, empty case names, duplicate case names, empty
/// tables, out-of-range row indices, duplicate row indices, and incomplete
/// row coverage (every table row `0..table_len` must be executed by exactly
/// one case). The compiler already rejects duplicate `#[test]` function
/// names; this runtime check additionally guards macro misuse (for example
/// an empty `cases` list, which would otherwise compile to a suite that runs
/// nothing, or two cases sharing one row while another row runs never).
pub(crate) fn validate_case_table(
    suite: &str,
    case_names: &[&str],
    row_indices: &[usize],
    table_len: usize,
) -> Result<(), String> {
    if suite.is_empty() {
        return Err("data-driven suite name must not be empty".to_string());
    }
    if case_names.is_empty() {
        return Err(format!(
            "data-driven suite {suite} must list at least one case"
        ));
    }
    if table_len == 0 {
        return Err(format!("data-driven suite {suite} has an empty case table"));
    }
    if row_indices.len() != case_names.len() {
        return Err(format!(
            "data-driven suite {suite} lists {} cases but {} row indices",
            case_names.len(),
            row_indices.len()
        ));
    }
    let mut seen = BTreeSet::new();
    for name in case_names {
        if name.is_empty() {
            return Err(format!("data-driven suite {suite} has an empty case name"));
        }
        if !seen.insert(*name) {
            return Err(format!(
                "data-driven suite {suite} lists duplicate case {name}"
            ));
        }
    }
    let mut seen_rows = BTreeSet::new();
    for (name, index) in case_names.iter().zip(row_indices.iter()) {
        if *index >= table_len {
            return Err(format!(
                "data-driven suite {suite} case {name} maps to row {index} \
                 outside 0..{table_len}"
            ));
        }
        if !seen_rows.insert(*index) {
            return Err(format!(
                "data-driven suite {suite} maps row {index} twice (duplicate index)"
            ));
        }
    }
    let covered: Vec<usize> = seen_rows.into_iter().collect();
    let expected: Vec<usize> = (0..table_len).collect();
    if covered != expected {
        let missing: Vec<usize> = expected
            .into_iter()
            .filter(|row| !covered.contains(row))
            .collect();
        return Err(format!(
            "data-driven suite {suite} omits table rows {missing:?} \
             ({}/{} rows covered)",
            covered.len(),
            table_len
        ));
    }
    Ok(())
}

/// Schema version of `engine-rust/tests/data-driven/mapping.json`.
pub(crate) const MAPPING_SCHEMA_VERSION: &str = "rust-data-driven-mapping-v1";

/// One old-test-to-new-cases entry of the machine-readable mapping.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MappingEntry {
    pub module: String,
    pub old_test: String,
    pub new_cases: Vec<String>,
    pub row_ids: Vec<String>,
    pub assertions: Vec<String>,
}

/// Parse and validate a mapping document. Every malformed shape fails closed
/// with an error instead of yielding a partial table.
pub(crate) fn parse_mapping(json: &str) -> Result<Vec<MappingEntry>, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| format!("mapping is not valid JSON: {error}"))?;
    let version = value
        .get("schemaVersion")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if version != MAPPING_SCHEMA_VERSION {
        return Err(format!("unsupported mapping schemaVersion {version:?}"));
    }
    let suites: Vec<MappingEntry> = serde_json::from_value(
        value
            .get("suites")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|error| format!("mapping suites are malformed: {error}"))?;
    validate_mapping_entries(&suites)?;
    Ok(suites)
}

/// Validate parsed mapping entries: non-empty suites, no duplicate old or new
/// ids, every entry carries its assertion mapping (missing expectations fail).
pub(crate) fn validate_mapping_entries(entries: &[MappingEntry]) -> Result<(), String> {
    if entries.is_empty() {
        return Err("mapping must list at least one suite".to_string());
    }
    let mut seen_old = BTreeSet::new();
    let mut seen_new = BTreeSet::new();
    for entry in entries {
        if entry.module.is_empty() || entry.old_test.is_empty() {
            return Err("mapping entry needs a module and oldTest".to_string());
        }
        if !seen_old.insert((entry.module.as_str(), entry.old_test.as_str())) {
            return Err(format!(
                "duplicate mapping entry for {}::{}",
                entry.module, entry.old_test
            ));
        }
        if entry.new_cases.is_empty() {
            return Err(format!(
                "mapping entry {} has no newCases (empty suite)",
                entry.old_test
            ));
        }
        if entry.assertions.is_empty() {
            return Err(format!(
                "mapping entry {} has no assertions (missing expectations)",
                entry.old_test
            ));
        }
        if entry.row_ids.len() != entry.new_cases.len() {
            return Err(format!(
                "mapping entry {} has {} rowIds for {} newCases",
                entry.old_test,
                entry.row_ids.len(),
                entry.new_cases.len()
            ));
        }
        for case in &entry.new_cases {
            if case.is_empty() {
                return Err(format!(
                    "mapping entry {} has an empty new case id",
                    entry.old_test
                ));
            }
            if !seen_new.insert(case.as_str()) {
                return Err(format!("duplicate new case id {case}"));
            }
        }
    }
    Ok(())
}

/// Look up the entry for one migrated old test. Unknown ids fail closed.
pub(crate) fn find_mapping_entry<'a>(
    entries: &'a [MappingEntry],
    module: &str,
    old_test: &str,
) -> Result<&'a MappingEntry, String> {
    entries
        .iter()
        .find(|entry| entry.module == module && entry.old_test == old_test)
        .ok_or_else(|| format!("unknown mapping entry {module}::{old_test}"))
}

const REAL_MAPPING_JSON: &str = include_str!("../../../tests/data-driven/mapping.json");

#[cfg(test)]
mod loader_tests {
    use super::*;

    fn entry(old_test: &str, new_cases: &[&str]) -> MappingEntry {
        MappingEntry {
            module: "replay::tests_example".to_string(),
            old_test: old_test.to_string(),
            new_cases: new_cases.iter().map(|case| case.to_string()).collect(),
            row_ids: new_cases.iter().map(|case| case.to_string()).collect(),
            assertions: vec!["state.p2.core.hp".to_string()],
        }
    }

    #[test]
    fn case_table_validator_accepts_a_unique_complete_suite() {
        assert!(validate_case_table("suite", &["case_a", "case_b"], &[0, 1], 2).is_ok());
        // Row order is irrelevant; only exact complete coverage matters.
        assert!(validate_case_table("suite", &["case_b", "case_a"], &[1, 0], 2).is_ok());
        assert!(validate_case_table("suite", &["only"], &[0], 1).is_ok());
    }

    #[test]
    fn case_table_validator_rejects_empty_suites_names_and_tables() {
        assert!(validate_case_table("suite", &[], &[], 2).is_err());
        assert!(validate_case_table("", &["case_a"], &[0], 1).is_err());
        assert!(validate_case_table("suite", &["case_a", ""], &[0, 1], 2).is_err());
        assert!(validate_case_table("suite", &["case_a"], &[0], 0).is_err());
    }

    #[test]
    fn case_table_validator_rejects_duplicate_case_ids() {
        let error = validate_case_table("suite", &["case_a", "case_a"], &[0, 1], 2)
            .expect_err("duplicate case ids must fail");
        assert!(error.contains("duplicate"), "unexpected error: {error}");
    }

    #[test]
    fn case_table_validator_rejects_duplicate_row_indices() {
        // Regression probe: two rows with first => 0 and second => 0 must
        // fail (row 1 would otherwise run never while all cases pass).
        let error = validate_case_table("suite", &["first", "second"], &[0, 0], 2)
            .expect_err("duplicate row indices must fail");
        assert!(error.contains("duplicate"), "unexpected error: {error}");
    }

    #[test]
    fn case_table_validator_rejects_omitted_rows() {
        let error =
            validate_case_table("suite", &["only"], &[0], 2).expect_err("omitted rows must fail");
        assert!(error.contains("omit"), "unexpected error: {error}");
        let error = validate_case_table("suite", &["a", "b"], &[0, 2], 3)
            .expect_err("omitted rows must fail");
        assert!(error.contains("[1]"), "unexpected error: {error}");
    }

    #[test]
    fn case_table_validator_rejects_out_of_range_indices() {
        let error = validate_case_table("suite", &["a", "b"], &[0, 2], 2)
            .expect_err("out-of-range indices must fail");
        assert!(error.contains("outside"), "unexpected error: {error}");
        assert!(validate_case_table("suite", &["a"], &[7], 2).is_err());
    }

    #[test]
    fn case_table_validator_rejects_index_count_mismatch() {
        assert!(validate_case_table("suite", &["a", "b"], &[0], 2).is_err());
        assert!(validate_case_table("suite", &["a"], &[0, 1], 2).is_err());
    }

    #[test]
    fn mapping_loader_rejects_malformed_json_and_schema() {
        assert!(parse_mapping("{not json").is_err());
        assert!(parse_mapping(r#"{"schemaVersion":"other","suites":[]}"#).is_err());
        assert!(parse_mapping(r#"{"schemaVersion":"rust-data-driven-mapping-v1"}"#).is_err());
        assert!(
            parse_mapping(r#"{"schemaVersion":"rust-data-driven-mapping-v1","suites":[]}"#)
                .is_err()
        );
    }

    #[test]
    fn mapping_loader_rejects_missing_expectations_and_empty_suites() {
        let mut no_assertions = entry("old_test", &["new_case"]);
        no_assertions.assertions.clear();
        assert!(validate_mapping_entries(std::slice::from_ref(&no_assertions)).is_err());

        let no_cases = MappingEntry {
            new_cases: Vec::new(),
            row_ids: Vec::new(),
            ..entry("old_test", &[])
        };
        assert!(validate_mapping_entries(std::slice::from_ref(&no_cases)).is_err());

        let mut id_mismatch = entry("old_test", &["new_a", "new_b"]);
        id_mismatch.row_ids.pop();
        assert!(validate_mapping_entries(std::slice::from_ref(&id_mismatch)).is_err());
    }

    #[test]
    fn mapping_loader_rejects_duplicate_old_and_new_ids() {
        let duplicate_old = vec![entry("old_test", &["new_a"]), entry("old_test", &["new_b"])];
        assert!(validate_mapping_entries(&duplicate_old).is_err());

        let duplicate_new = vec![entry("old_a", &["new_a"]), entry("old_b", &["new_a"])];
        let error =
            validate_mapping_entries(&duplicate_new).expect_err("duplicate new ids must fail");
        assert!(error.contains("duplicate"), "unexpected error: {error}");
    }

    #[test]
    fn mapping_lookup_fails_closed_on_unknown_entries() {
        let entries = vec![entry("old_test", &["new_case"])];
        assert!(find_mapping_entry(&entries, "replay::tests_example", "old_test").is_ok());
        assert!(find_mapping_entry(&entries, "replay::tests_example", "missing").is_err());
        assert!(find_mapping_entry(&entries, "replay::tests_other", "old_test").is_err());
    }

    #[test]
    fn committed_mapping_parses_and_validates() {
        let entries = parse_mapping(REAL_MAPPING_JSON).expect("mapping.json validates");
        assert!(
            !entries.is_empty(),
            "mapping must document at least one migration"
        );
    }
}
