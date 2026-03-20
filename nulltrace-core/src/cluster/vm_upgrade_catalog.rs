//! Allowed VM upgrade tiers and prices (My Computer). Prices in USD cents.
//! Mirrors nullcloudData CPU_UPGRADES, RAM_UPGRADES, DISK_UPGRADES.

/// (value, price_cents). CPU: value = cores.
const CPU_TIERS: &[(i16, i64)] = &[
    (2, 0),
    (4, 4900),
    (6, 9900),
    (8, 14900),
    (10, 19900),
    (12, 24900),
    (16, 32900),
    (24, 44900),
];

/// (value, price_cents). RAM: value = GiB.
const RAM_TIERS: &[(i32, i64)] = &[
    (8, 0),
    (16, 3900),
    (32, 8900),
    (64, 17900),
    (96, 24900),
    (128, 31900),
    (192, 44900),
    (256, 57900),
];

/// (value, price_cents). Disk: value = GiB.
const DISK_TIERS: &[(i32, i64)] = &[
    (100, 0),
    (250, 2900),
    (500, 5900),
    (1000, 9900),
    (1500, 13900),
    (2000, 17900),
    (3000, 24900),
    (4000, 31900),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeType {
    Cpu,
    Ram,
    Disk,
}

impl UpgradeType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cpu" => Some(Self::Cpu),
            "ram" => Some(Self::Ram),
            "disk" => Some(Self::Disk),
            _ => None,
        }
    }
}

/// Returns price in cents for the given tier if it exists in the catalog.
pub fn get_price_cents(upgrade_type: UpgradeType, new_value: i32) -> Option<i64> {
    match upgrade_type {
        UpgradeType::Cpu => CPU_TIERS
            .iter()
            .find(|(v, _)| *v == new_value as i16)
            .map(|(_, p)| *p),
        UpgradeType::Ram => RAM_TIERS
            .iter()
            .find(|(v, _)| *v == new_value)
            .map(|(_, p)| *p),
        UpgradeType::Disk => DISK_TIERS
            .iter()
            .find(|(v, _)| *v == new_value)
            .map(|(_, p)| *p),
    }
}
