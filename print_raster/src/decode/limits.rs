#[derive(Clone, Debug)]
pub struct Limits {
    // The maximum number of bytes to decode per line, using for creating line buffer.
    pub bytes_per_line: u64,
    // The maximum number of bytes to decode per page.
    pub bytes_per_page: u64,
}

impl Limits {
    pub const NO_LIMITS: &Self = &Self {
        bytes_per_line: u64::MAX,
        bytes_per_page: u64::MAX,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::NO_LIMITS.clone()
    }
}
