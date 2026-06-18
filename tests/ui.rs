#[test]
fn dec_macro_rejects_non_zero_digits_past_scale() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/dec_too_many_digits.rs");
}
