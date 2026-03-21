//! Nominal-to-real resource mapping for VM scaling.
//!
//! Players see nominal values (GiB) in the UI; the server enforces smaller real limits (MB)
//! to scale to thousands of VMs on limited hardware.

/// Ratio: 1 GiB nominal = 1 MB real. For 5000 VMs × 16 GiB nominal → 80 MB total RAM.
/// Tune RAM_RATIO for your server: e.g. 4096 = 16 GiB nominal → 4 MB real.
const RAM_NOMINAL_TO_REAL_RATIO: i32 = 1024;

/// Ratio: 1 GiB nominal = 1 MB real. 50 GiB nominal → 50 MB real disk per VM.
const DISK_NOMINAL_TO_REAL_RATIO: i32 = 1024;

/// Minimum real RAM bytes per VM (fallback when nominal is very small).
const MIN_REAL_RAM_BYTES: usize = 1024 * 1024; // 1 MB

/// Minimum real disk bytes per VM (fallback when nominal is very small).
const MIN_REAL_DISK_BYTES: i64 = 1024 * 1024; // 1 MB

/// Converts nominal RAM (MB, e.g. 16384 = 16 GiB) to real Lua heap limit in bytes.
/// Example: 16384 MB nominal → 16 MB real (with ratio 1024).
pub fn nominal_ram_mb_to_real_bytes(nominal_mb: i32) -> usize {
    let real_mb = (nominal_mb / RAM_NOMINAL_TO_REAL_RATIO).max(1);
    let bytes = (real_mb as usize) * 1024 * 1024;
    bytes.max(MIN_REAL_RAM_BYTES)
}

/// Converts nominal disk (MB, e.g. 51200 = 50 GiB) to real PostgreSQL limit in bytes.
/// Example: 51200 MB nominal → 50 MB real (with ratio 1024).
pub fn nominal_disk_mb_to_real_bytes(nominal_mb: i32) -> i64 {
    let real_mb = (nominal_mb / DISK_NOMINAL_TO_REAL_RATIO).max(1);
    let bytes = (real_mb as i64) * 1024 * 1024;
    bytes.max(MIN_REAL_DISK_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nominal_ram_16_gib() {
        // 16 GiB = 16384 MB nominal → 16 MB real (ratio 1024)
        let bytes = nominal_ram_mb_to_real_bytes(16384);
        assert_eq!(bytes, 16 * 1024 * 1024);
    }

    #[test]
    fn test_nominal_ram_50_gib() {
        // 50 GiB = 51200 MB nominal → 50 MB real
        let bytes = nominal_ram_mb_to_real_bytes(51200);
        assert_eq!(bytes, 50 * 1024 * 1024);
    }

    #[test]
    fn test_nominal_ram_small_uses_minimum() {
        let bytes = nominal_ram_mb_to_real_bytes(512);
        assert_eq!(bytes, MIN_REAL_RAM_BYTES);
    }

    #[test]
    fn test_nominal_disk_50_gib() {
        // 50 GiB = 51200 MB nominal → 50 MB real
        let bytes = nominal_disk_mb_to_real_bytes(51200);
        assert_eq!(bytes, 50 * 1024 * 1024);
    }

    #[test]
    fn test_nominal_disk_small_uses_minimum() {
        let bytes = nominal_disk_mb_to_real_bytes(512);
        assert_eq!(bytes, MIN_REAL_DISK_BYTES);
    }
}
