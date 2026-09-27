//! Tests for the §24 scientific references registry check
//! (`src/validate/references.rs`).

use oss_spec::validate::{self, EVIDENCE_KINDS, citation_tags, entry_problems, is_bare_doi};
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn v24(report: &validate::Report) -> Vec<String> {
    report
        .violations
        .iter()
        .filter(|v| v.spec_section == "§24")
        .map(|v| v.message.clone())
        .collect()
}

fn write(root: &Path, rel: &str, content: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

/// A complete, valid entry cited from `used_by`.
fn entry(used_by: &[&str]) -> serde_json::Value {
    json!({
        "evidence": "systematic-review",
        "authors": ["Galland BC", "Taylor BJ"],
        "title": "Normal sleep patterns in infants and children",
        "container": "Sleep Medicine Reviews",
        "year": 2012,
        "doi": "10.1016/j.smrv.2011.06.001",
        "language": "en",
        "quotes": [{ "text": "≈6 months 12.9 (8.8–17.0)", "at": "table 2" }],
        "supports": "The observed range of sleep at each age.",
        "usedBy": used_by,
    })
}

fn registry(entries: serde_json::Value) -> String {
    serde_json::to_string_pretty(&json!({ "references": entries })).unwrap()
}

/// A project that opted in and conforms: a tag in `src/sleep.ts`, its
/// entry, and a module that reads the registry for the about screen.
fn conforming(root: &Path) {
    write(
        root,
        "src/sleep.ts",
        "// 12.9 hours at six months [ref:galland-2012].\nexport const H = 12.9;\n",
    );
    write(
        root,
        "src/references.ts",
        "import registry from \"../docs/references.json\";\nexport const REFS = registry;\n",
    );
    write(
        root,
        "docs/references.json",
        &registry(json!({ "galland-2012": entry(&["src/sleep.ts"]) })),
    );
}

#[test]
fn project_without_registry_or_tags_is_not_checked() {
    let dir = tempdir().unwrap();
    write(dir.path(), "src/lib.rs", "pub fn f() {}\n");
    let report = validate::run(dir.path()).unwrap();
    assert!(v24(&report).is_empty(), "{:?}", v24(&report));
}

#[test]
fn conforming_project_passes() {
    let dir = tempdir().unwrap();
    conforming(dir.path());
    let report = validate::run(dir.path()).unwrap();
    assert!(v24(&report).is_empty(), "{:?}", v24(&report));
}

#[test]
fn tags_without_a_registry_are_a_violation() {
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "src/a.ts",
        "// [ref:who-2006] and [ref:luo-1998]\n",
    );
    let vs = v24(&validate::run(dir.path()).unwrap());
    assert_eq!(vs.len(), 1, "{vs:?}");
    assert!(vs[0].contains("luo-1998, who-2006"), "{}", vs[0]);
    assert!(vs[0].contains("docs/references.json"), "{}", vs[0]);
}

#[test]
fn a_tag_with_no_entry_is_a_violation() {
    let dir = tempdir().unwrap();
    conforming(dir.path());
    write(dir.path(), "src/growth.ts", "// [ref:who-2006]\n");
    let vs = v24(&validate::run(dir.path()).unwrap());
    assert_eq!(vs.len(), 1, "{vs:?}");
    assert!(
        vs[0].contains("[ref:who-2006] is cited in src/growth.ts"),
        "{}",
        vs[0]
    );
}

#[test]
fn an_entry_nothing_cites_is_a_violation() {
    let dir = tempdir().unwrap();
    conforming(dir.path());
    write(
        dir.path(),
        "docs/references.json",
        &registry(json!({
            "galland-2012": entry(&["src/sleep.ts"]),
            "iglowstein-2003": entry(&[]),
        })),
    );
    let vs = v24(&validate::run(dir.path()).unwrap());
    assert_eq!(vs.len(), 1, "{vs:?}");
    assert!(
        vs[0].contains("`iglowstein-2003` is not cited"),
        "{}",
        vs[0]
    );
}

#[test]
fn used_by_must_list_exactly_the_citing_files() {
    let dir = tempdir().unwrap();
    conforming(dir.path());
    write(dir.path(), "src/copy.ts", "// [ref:galland-2012]\n");
    let vs = v24(&validate::run(dir.path()).unwrap());
    assert_eq!(vs.len(), 1, "{vs:?}");
    assert!(
        vs[0].contains("usedBy [src/sleep.ts]") && vs[0].contains("[src/copy.ts, src/sleep.ts]"),
        "{}",
        vs[0]
    );
}

#[test]
fn tags_in_tests_and_docs_do_not_count() {
    let dir = tempdir().unwrap();
    conforming(dir.path());
    // Neither is in the §20.5 source tree, so neither needs an entry or a
    // place in `usedBy`.
    write(
        dir.path(),
        "tests/sleep_test.ts",
        "// [ref:galland-2012] [ref:x-1]\n",
    );
    write(dir.path(), "src/sleep.test.ts", "// [ref:x-2]\n");
    write(
        dir.path(),
        "docs/sleep.md",
        "See [ref:galland-2012] and [ref:x-3].\n",
    );
    let report = validate::run(dir.path()).unwrap();
    assert!(v24(&report).is_empty(), "{:?}", v24(&report));
}

#[test]
fn invalid_json_is_a_violation() {
    let dir = tempdir().unwrap();
    conforming(dir.path());
    write(dir.path(), "docs/references.json", "{ \"references\": ");
    let vs = v24(&validate::run(dir.path()).unwrap());
    assert_eq!(vs.len(), 1, "{vs:?}");
    assert!(vs[0].contains("not valid JSON"), "{}", vs[0]);
}

#[test]
fn a_registry_without_a_references_object_is_a_violation() {
    let dir = tempdir().unwrap();
    conforming(dir.path());
    write(dir.path(), "docs/references.json", "[]");
    let vs = v24(&validate::run(dir.path()).unwrap());
    assert_eq!(vs.len(), 1, "{vs:?}");
    assert!(vs[0].contains("`references` object"), "{}", vs[0]);
}

#[test]
fn a_registry_nothing_reads_is_a_violation() {
    let dir = tempdir().unwrap();
    conforming(dir.path());
    fs::remove_file(dir.path().join("src/references.ts")).unwrap();
    // A test that reads it is not a user-facing view.
    write(
        dir.path(),
        "tests/references_test.ts",
        "readFileSync(\"docs/references.json\")\n",
    );
    let vs = v24(&validate::run(dir.path()).unwrap());
    assert_eq!(vs.len(), 1, "{vs:?}");
    assert!(vs[0].contains("§24.4"), "{}", vs[0]);
}

#[test]
fn a_build_script_that_reads_the_registry_counts() {
    let dir = tempdir().unwrap();
    conforming(dir.path());
    fs::remove_file(dir.path().join("src/references.ts")).unwrap();
    write(
        dir.path(),
        "scripts/gen-references.mjs",
        "const refs = JSON.parse(readFileSync(\"docs/references.json\"));\n",
    );
    let report = validate::run(dir.path()).unwrap();
    assert!(v24(&report).is_empty(), "{:?}", v24(&report));
}

#[test]
fn a_complete_entry_has_no_problems() {
    assert!(entry_problems("galland-2012", &entry(&["src/a.ts"])).is_empty());
}

#[test]
fn an_organization_and_a_url_are_enough() {
    let e = json!({
        "evidence": "guideline",
        "organization": "World Health Organization",
        "title": "Guidelines on physical activity, sedentary behaviour and sleep for children under 5 years of age",
        "year": 2019,
        "url": "https://iris.who.int/handle/10665/311664",
        "accessed": "2026-09-27",
        "quotes": [{ "text": "12–16h of good quality sleep" }],
        "supports": "The recommended total sleep.",
        "usedBy": ["src/sleep.ts"],
    });
    assert!(entry_problems("who-2019-under5", &e).is_empty());
}

#[test]
fn every_missing_field_is_named() {
    let problems = entry_problems("Bad_Id", &json!({ "year": "2012" })).join("\n");
    for needle in [
        "kebab-case",
        "`title`",
        "`year`",
        "`authors`",
        "`doi`, `url`, or `isbn`",
        "`evidence`",
        "`quotes`",
        "`supports`",
        "`usedBy`",
    ] {
        assert!(problems.contains(needle), "{needle} not in:\n{problems}");
    }
}

#[test]
fn malformed_fields_are_named() {
    let mut e = entry(&["src/a.ts"]);
    e["doi"] = json!("https://doi.org/10.1016/x");
    e["url"] = json!("http://example.org");
    e["accessed"] = json!("27 Sep 2026");
    e["evidence"] = json!("anecdote");
    e["quotes"] = json!([{ "text": "  " }]);
    let problems = entry_problems("x-2012", &e).join("\n");
    for needle in [
        "bare DOI",
        "https:// URL",
        "YYYY-MM-DD",
        "anecdote",
        "`quotes`",
    ] {
        assert!(problems.contains(needle), "{needle} not in:\n{problems}");
    }
}

#[test]
fn the_evidence_vocabulary_is_the_specs() {
    assert_eq!(
        EVIDENCE_KINDS,
        [
            "guideline",
            "consensus",
            "systematic-review",
            "meta-analysis",
            "randomized-trial",
            "cohort",
            "clinical-study",
            "review",
            "method",
            "dataset",
            "health-service",
        ]
    );
}

#[test]
fn citation_tags_skip_placeholders() {
    let text = "a `[ref:<id>]` tag, [ref:who-2006] and [ref:Bad] [ref:-x] [ref:a--b] \
                [ref:luo-1998][ref:open";
    assert_eq!(citation_tags(text), ["who-2006", "luo-1998"]);
}

#[test]
fn bare_dois() {
    assert!(is_bare_doi("10.1016/j.smrv.2011.06.001"));
    assert!(is_bare_doi("10.5664/jcsm.5866"));
    assert!(!is_bare_doi("https://doi.org/10.1016/x"));
    assert!(!is_bare_doi("10.12/x"));
    assert!(!is_bare_doi("10.1016/"));
    assert!(!is_bare_doi("10.1016/a b"));
}
