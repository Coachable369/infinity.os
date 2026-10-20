//! Versioned clock checkpoints share the installer-created native System object.
use super::*;

impl<D: BlockDevice> ObjectStore<D> {
    // ------------------------=
    // FUNC: clock_correction
    // DESC: Reads the persistent UTC correction; original installer records have no correction.
    // ------------------=
    pub fn clock_correction(&mut self) -> Option<i64> {
        self.date_time_configuration()?;
        let id = self.resolve(b"/system/settings/date-time").ok()?;
        let mut bytes = [0; 32];
        if self.read(id, None, &mut bytes).ok()? != bytes.len() { return None; }
        if u16::from_le_bytes([bytes[8], bytes[9]]) == 1 { return Some(0); }
        let seconds = i64::from_le_bytes(bytes[24..32].try_into().ok()?);
        (-6_311_520_000..=6_311_520_000).contains(&seconds).then_some(seconds)
    }

    // ------------------------=
    // FUNC: update_clock_configuration
    // DESC: Atomically checkpoints local projection and UTC correction with bounded history and no duplicate writes.
    // ------------------=
    pub fn update_clock_configuration(&mut self, configuration: DateTimeConfiguration,
        correction_seconds: i64) -> Result<(), ObjectError> {
        if !configuration.is_valid() || !(-6_311_520_000..=6_311_520_000).contains(&correction_seconds) {
            return Err(ObjectError::InvalidObject);
        }
        let id = self.resolve(b"/system/settings/date-time")?;
        // Reject corrupt records rather than silently replacing unknown formats.
        self.date_time_configuration().ok_or(ObjectError::InvalidObject)?;
        self.clock_correction().ok_or(ObjectError::InvalidObject)?;
        let mut previous = [0; 32];
        if self.read(id, None, &mut previous)? != previous.len() { return Err(ObjectError::InvalidObject); }
        let mut bytes = previous;
        bytes[8..10].copy_from_slice(&2u16.to_le_bytes());
        bytes[12..14].copy_from_slice(&configuration.year.to_le_bytes());
        bytes[14] = configuration.month; bytes[15] = configuration.day;
        bytes[16] = configuration.hour; bytes[17] = configuration.minute; bytes[18] = configuration.second;
        bytes[19] = 2; // Runtime-authorized system wall clock.
        bytes[20..22].copy_from_slice(&configuration.time_zone_id.to_le_bytes());
        bytes[22..24].copy_from_slice(&configuration.utc_offset_minutes.to_le_bytes());
        bytes[24..32].copy_from_slice(&correction_seconds.to_le_bytes());
        if bytes != previous { self.replace_state(id, &bytes)?; }
        Ok(())
    }
}
