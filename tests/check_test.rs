use oss_spec::check::{check_toolchain_versions, version_ge};

#[test]
fn version_ge_pads_shorter_segments() {
    assert_eq!(version_ge("1.82", "1.82.0"), Some(true));
    assert_eq!(version_ge("1.82.0", "1.82"), Some(true));
    assert_eq!(version_ge("1.83", "1.82.0"), Some(true));
    assert_eq!(version_ge("1.81.9", "1.82.0"), Some(false));
    assert_eq!(version_ge("22", "22"), Some(true));
    assert_eq!(version_ge("21", "22"), Some(false));
}

#[test]
fn rust_pinned_exact_minimum_is_ok() {
    let yml = "      - uses: dtolnay/rust-toolchain@1.82.0\n";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn rust_pinned_above_minimum_is_ok() {
    let yml = "      - uses: dtolnay/rust-toolchain@1.85.0\n";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn rust_stable_is_floating() {
    let yml = "      - uses: dtolnay/rust-toolchain@stable\n";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].spec_section, "§10.3");
    assert!(v[0].message.contains("Rust"));
    assert!(v[0].message.contains("floating specifier 'stable'"));
}

#[test]
fn rust_below_minimum_is_violation() {
    let yml = "      - uses: dtolnay/rust-toolchain@1.75.0\n";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("1.75.0"));
    assert!(v[0].message.contains("minimum is 1.82.0"));
}

#[test]
fn node_lts_star_is_floating() {
    let yml = "\
      - uses: actions/setup-node@v4
        with:
          node-version: \"lts/*\"
";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("Node"));
    assert!(v[0].message.contains("floating specifier 'lts/*'"));
}

#[test]
fn node_22_is_ok() {
    let yml = "\
      - uses: actions/setup-node@v4
        with:
          node-version: \"22\"
";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn node_20_is_below_minimum() {
    let yml = "\
      - uses: actions/setup-node@v4
        with:
          node-version: \"20\"
";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("Node"));
    assert!(v[0].message.contains("pinned to 20"));
    assert!(v[0].message.contains("minimum is 22"));
}

#[test]
fn python_312_is_ok() {
    let yml = "\
      - uses: actions/setup-python@v5
        with:
          python-version: \"3.12\"
";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn python_311_is_below_minimum() {
    let yml = "\
      - uses: actions/setup-python@v5
        with:
          python-version: \"3.11\"
";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("Python"));
}

#[test]
fn go_stable_is_floating() {
    let yml = "\
      - uses: actions/setup-go@v5
        with:
          go-version: \"stable\"
";
    let v = check_toolchain_versions("ci.yml", yml);
    assert_eq!(v.len(), 1);
    assert!(v[0].message.contains("Go"));
    assert!(v[0].message.contains("floating specifier 'stable'"));
}

#[test]
fn go_122_is_ok() {
    let yml = "\
      - uses: actions/setup-go@v5
        with:
          go-version: \"1.22\"
";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn missing_toolchain_block_is_not_a_violation() {
    let yml =
        "name: CI\njobs:\n  noop:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo hi\n";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}

#[test]
fn rust_short_version_is_accepted() {
    // `@1.82` (no patch) should be treated as `1.82.0` and accepted.
    let yml = "      - uses: dtolnay/rust-toolchain@1.82\n";
    assert!(check_toolchain_versions("ci.yml", yml).is_empty());
}
