#[test]
fn invalid_macro_usage_reports_good_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
