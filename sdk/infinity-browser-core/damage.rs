//! Browser chrome changes must not invalidate an unchanged engine surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PageKey {
    pub frame: u64,
    pub tab: u32,
    pub error: u32,
    pub permission: u8,
    pub download: u8,
    pub download_content: u64,
    pub loading: bool,
    pub busy: bool,
    pub status:u64,
}

// ------------------------=
// FUNC: chrome_only
// DESC: Reuses page and status pixels only when every content-affecting projection matches a previously painted surface.
// ------------------=
pub fn chrome_only(previous: Option<PageKey>, current: PageKey) -> bool {
    previous == Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: page_changes_require_full_surface_update
    // DESC: Verifies first paint and every content/status transition reject chrome-only invalidation.
    // ------------------=
    #[test]
    fn page_changes_require_full_surface_update() {
        let base=PageKey::default();
        assert!(!chrome_only(None,base));
        assert!(chrome_only(Some(base),base));
        for changed in [PageKey{frame:1,..base},PageKey{tab:1,..base},
            PageKey{error:1,..base},PageKey{permission:1,..base},
            PageKey{download:1,..base},PageKey{download_content:1,..base},PageKey{loading:true,..base},PageKey{busy:true,..base},PageKey{status:1,..base}] {
            assert!(!chrome_only(Some(base),changed));
        }
    }
}
