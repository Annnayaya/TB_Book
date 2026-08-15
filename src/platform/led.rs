use std::fs;
use std::path::Path;

pub struct LedController;

impl LedController {
    /// Set Trimui Brick RGB LEDs color (RGB values 0..255)
    pub fn set_rgb(r: u8, g: u8, b: u8) {
        #[cfg(target_os = "linux")]
        {
            let _ = Self::write_sysfs("/sys/class/leds/red/brightness", &r.to_string());
            let _ = Self::write_sysfs("/sys/class/leds/green/brightness", &g.to_string());
            let _ = Self::write_sysfs("/sys/class/leds/blue/brightness", &b.to_string());
        }
        let _ = (r, g, b);
    }

    pub fn turn_off() {
        Self::set_rgb(0, 0, 0);
    }

    #[allow(dead_code)]
    fn write_sysfs(path: &str, val: &str) -> bool {
        if Path::new(path).exists() {
            fs::write(path, val).is_ok()
        } else {
            false
        }
    }
}
