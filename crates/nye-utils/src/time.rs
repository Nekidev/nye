use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current time in milliseconds since the UNIX epoch.
///
/// This function retrieves the current system time, calculates the duration
/// since the UNIX epoch, and converts it to milliseconds. It is useful for
/// timestamping events or measuring time intervals in applications.
///
/// Returns:
/// * `u64` - The current time in milliseconds since the UNIX epoch.
///
/// Panics:
/// * If the system time is before the UNIX epoch, which is unlikely but
///   possible on some systems.
///
/// Returns:
/// [`u64`] - The current time in milliseconds since the UNIX epoch.
pub fn utc_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

/// Returns the current time as a [`std::time::Duration`] since the UNIX epoch.
///
/// Returns:
/// [`std::time::Duration`] - The current time as a duration since the UNIX
/// epoch.
pub fn utc_now_duration() -> std::time::Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
}
