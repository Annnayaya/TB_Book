pub struct RumbleController;

impl RumbleController {
    /// Trigger micro tactile haptic vibration (duration in ms)
    pub fn pulse(duration_ms: u32) {
        #[cfg(target_os = "linux")]
        {
            let vibrator_path = "/sys/class/timed_output/vibrator/enable";
            if std::path::Path::new(vibrator_path).exists() {
                let _ = std::fs::write(vibrator_path, duration_ms.to_string());
            }
        }
        let _ = duration_ms;
    }
}
