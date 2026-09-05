pub trait BlockDevice {
    // ------------------------=
    // FUNC: block_count
    // DESC: Implements the block count operation.
    // ------------------=
    fn block_count(&self) -> u64;
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads read sector data.
    // ------------------=
    fn read_sector(&mut self, lba: u64, sector: &mut [u8; 512]) -> bool;
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes or updates write sector data.
    // ------------------=
    fn write_sector(&mut self, lba: u64, sector: &[u8; 512]) -> bool;
    // ------------------------=
    // FUNC: flush
    // DESC: Implements the flush operation.
    // ------------------=
    fn flush(&mut self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTimeConfiguration {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub time_zone_id: u16,
    pub utc_offset_minutes: i16,
}

impl DateTimeConfiguration {
    // ------------------------=
    // FUNC: is_valid
    // DESC: Mirrors the kernel-side bounds used to validate installed date-and-time metadata.
    // ------------------=
    pub fn is_valid(&self) -> bool {
        (2020..=2199).contains(&self.year)
            && (1..=12).contains(&self.month)
            && self.day >= 1
            && self.day <= days_in_month(self.year, self.month)
            && self.hour <= 23
            && self.minute <= 59
            && self.second <= 59
            && self.time_zone_id >= 1
            && (-14 * 60..=14 * 60).contains(&self.utc_offset_minutes)
    }
}

// ------------------------=
// FUNC: days_in_month
// DESC: Mirrors the kernel Gregorian month bounds for host-side object validation.
// ------------------=
pub const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[path = "../kernel/storage/object.rs"]
pub mod object;
#[path = "../kernel/storage/organization.rs"]
pub mod organization;
#[path = "../kernel/storage/layout.rs"]
pub mod layout;
