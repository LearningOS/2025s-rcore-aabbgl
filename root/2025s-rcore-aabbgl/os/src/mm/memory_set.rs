
/// address space
pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    /// Create a new `MemorySet` from a given token.
    ///
    /// This method initializes a new `MemorySet` using the provided token.
    /// It assumes that the token corresponds to a valid page table.
    pub fn from_token(token: usize) -> Self {
        let page_table = PageTable::from_token(token);
        Self {
            page_table,
            areas: Vec::new(),
        }
    }

    pub fn unmap(&mut self, vpn: VirtPageNum) {
        for area in &mut self.areas {
            if area.vpn_range.contains(&vpn) {
                area.unmap_one(&mut self.page_table, vpn);
                return;
            }
        }
        warn!("Attempted to unmap an unmapped virtual page number: {:?}", vpn);
    }

}
