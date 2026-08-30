#[allow(dead_code)]
pub mod mappings;
#[allow(dead_code)]
pub mod runner;
#[allow(dead_code)]
pub mod secrets;

#[allow(dead_code)]
pub mod assertions {
    pub fn assert_nonempty(bytes: &[u8], label: &str) {
        assert!(!bytes.is_empty(), "expected nonempty bytes for {label}");
    }

    pub fn assert_starts_with(bytes: &[u8], prefix: &[u8], label: &str) {
        assert!(bytes.starts_with(prefix), "unexpected prefix for {label}");
    }
}
