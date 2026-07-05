#[test]
fn test_crate_links() {
    assert_eq!(env!("CARGO_PKG_NAME"), "openakb-validate");
}
