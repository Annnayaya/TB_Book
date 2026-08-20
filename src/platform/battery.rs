use std::fs;
use std::path::{Path, PathBuf};

const PRIMARY_BATTERY_PATHS: &[&str] = &[
    "/sys/class/power_supply/axp2202-battery",
    "/sys/class/power_supply/battery",
    "/sys/class/power_supply/BAT0",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryStatus {
    pub percent: u8,
    pub charging: bool,
}

pub fn read_battery_status() -> Option<BatteryStatus> {
    for path in PRIMARY_BATTERY_PATHS {
        if let Some(status) = read_battery_dir(Path::new(path)) {
            return Some(status);
        }
    }

    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if is_battery_dir(&path) {
            if let Some(status) = read_battery_dir(&path) {
                return Some(status);
            }
        }
    }

    None
}

fn is_battery_dir(path: &Path) -> bool {
    let supply_type = read_trimmed(path.join("type"));
    if supply_type
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("battery"))
    {
        return true;
    }

    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            lower.contains("battery") || lower == "bat0"
        })
        .unwrap_or(false)
}

fn read_battery_dir(path: &Path) -> Option<BatteryStatus> {
    let percent = parse_capacity(&read_trimmed(path.join("capacity"))?)?;
    let charging = read_trimmed(path.join("status"))
        .as_deref()
        .map(is_charging_status)
        .unwrap_or(false);

    Some(BatteryStatus { percent, charging })
}

fn read_trimmed(path: PathBuf) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn parse_capacity(value: &str) -> Option<u8> {
    let percent = value.trim().parse::<u8>().ok()?;
    (percent <= 100).then_some(percent)
}

fn is_charging_status(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("charging")
}

#[cfg(test)]
mod tests {
    use super::{is_charging_status, parse_capacity};

    #[test]
    fn parses_only_valid_battery_percentages() {
        assert_eq!(parse_capacity("85\n"), Some(85));
        assert_eq!(parse_capacity("0"), Some(0));
        assert_eq!(parse_capacity("100"), Some(100));
        assert_eq!(parse_capacity("101"), None);
        assert_eq!(parse_capacity("unknown"), None);
    }

    #[test]
    fn charging_status_is_not_confused_with_not_charging() {
        assert!(is_charging_status("Charging\n"));
        assert!(!is_charging_status("Not charging"));
        assert!(!is_charging_status("Discharging"));
    }
}
