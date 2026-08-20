use crate::input::{HandheldButton, InputState};
use std::collections::{HashMap, HashSet};

#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;
#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct RawInputEvent {
    pub time_sec: usize,
    pub time_usec: usize,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

pub const EV_KEY: u16 = 0x01;
pub const EV_ABS: u16 = 0x03;

const ABS_Z: u16 = 0x02;
const ABS_RZ: u16 = 0x05;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;

pub struct LinuxInputManager {
    #[cfg(target_os = "linux")]
    devices: Vec<File>,
    held_buttons: HashMap<HandheldButton, u32>,
    active_held: HashSet<HandheldButton>,
}

impl LinuxInputManager {
    pub fn new() -> Self {
        #[cfg(target_os = "linux")]
        {
            let mut devices = Vec::new();
            for i in 0..16 {
                let path = format!("/dev/input/event{}", i);
                if let Ok(file) = std::fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NONBLOCK)
                    .open(&path)
                {
                    println!("✓ 成功监听输入设备: {}", path);
                    devices.push(file);
                }
            }

            if devices.is_empty() {
                eprintln!("⚠️ 未检测到 /dev/input/event* 设备");
            }

            Self {
                devices,
                held_buttons: HashMap::new(),
                active_held: HashSet::new(),
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            Self {
                held_buttons: HashMap::new(),
                active_held: HashSet::new(),
            }
        }
    }

    pub fn update(&mut self) -> InputState {
        let mut pressed = Vec::new();
        let mut released = Vec::new();

        #[cfg(target_os = "linux")]
        {
            let event_size = std::mem::size_of::<RawInputEvent>();
            // Read directly into an aligned event array. Reinterpreting a
            // byte-aligned [u8] buffer as input_event would be undefined
            // behavior on AArch64 when the address is not naturally aligned.
            let mut event_buffer = [RawInputEvent::default(); 32];

            // Borrow the fields independently so input events can update the
            // shared button state while each device file is being drained.
            let active_held = &mut self.active_held;
            let held_buttons = &mut self.held_buttons;

            for dev in &mut self.devices {
                let fd = dev.as_raw_fd();
                loop {
                    let bytes_read = unsafe {
                        libc::read(
                            fd,
                            event_buffer.as_mut_ptr() as *mut libc::c_void,
                            event_buffer.len() * event_size,
                        )
                    };

                    if bytes_read <= 0 {
                        break;
                    }

                    let num_events = (bytes_read as usize) / event_size;
                    for ev in &event_buffer[..num_events] {
                        process_event(
                            ev.type_,
                            ev.code,
                            ev.value,
                            active_held,
                            held_buttons,
                            &mut pressed,
                            &mut released,
                        );
                    }
                }
            }
        }

        // Apply repeat acceleration for held navigation keys
        for (&btn, frames) in self.held_buttons.iter_mut() {
            *frames += 1;
            if *frames > 15 && (*frames % 4 == 0) {
                pressed.push(btn);
            }
        }

        let held: Vec<HandheldButton> = self.active_held.iter().copied().collect();

        InputState {
            keys_pressed: pressed,
            keys_held: held,
            keys_released: released,
        }
    }
}

fn process_event(
    event_type: u16,
    code: u16,
    value: i32,
    active_held: &mut HashSet<HandheldButton>,
    held_buttons: &mut HashMap<HandheldButton, u32>,
    pressed: &mut Vec<HandheldButton>,
    released: &mut Vec<HandheldButton>,
) {
    match event_type {
        EV_KEY => {
            if let Some(button) = map_evdev_code_to_button(code) {
                match value {
                    1 => press_button(button, active_held, held_buttons, pressed),
                    0 => release_button(button, active_held, held_buttons, released),
                    // The application supplies deterministic frame-based key
                    // repeat, so kernel repeat events must not be counted twice.
                    2 => {}
                    _ => {}
                }
            }
        }
        EV_ABS => match code {
            // TrimUI Brick exposes its D-pad as HAT axes rather than EV_KEY.
            ABS_HAT0X => update_axis_pair(
                value,
                HandheldButton::DpadLeft,
                HandheldButton::DpadRight,
                active_held,
                held_buttons,
                pressed,
                released,
            ),
            ABS_HAT0Y => update_axis_pair(
                value,
                HandheldButton::DpadUp,
                HandheldButton::DpadDown,
                active_held,
                held_buttons,
                pressed,
                released,
            ),
            // The analog triggers report 0 when released and a positive value
            // (normally 255 on the Brick) when pressed.
            ABS_Z => update_trigger(
                HandheldButton::L2,
                value,
                active_held,
                held_buttons,
                pressed,
                released,
            ),
            ABS_RZ => update_trigger(
                HandheldButton::R2,
                value,
                active_held,
                held_buttons,
                pressed,
                released,
            ),
            _ => {}
        },
        _ => {}
    }
}

fn update_axis_pair(
    value: i32,
    negative: HandheldButton,
    positive: HandheldButton,
    active_held: &mut HashSet<HandheldButton>,
    held_buttons: &mut HashMap<HandheldButton, u32>,
    pressed: &mut Vec<HandheldButton>,
    released: &mut Vec<HandheldButton>,
) {
    if value < 0 {
        release_button(positive, active_held, held_buttons, released);
        press_button(negative, active_held, held_buttons, pressed);
    } else if value > 0 {
        release_button(negative, active_held, held_buttons, released);
        press_button(positive, active_held, held_buttons, pressed);
    } else {
        release_button(negative, active_held, held_buttons, released);
        release_button(positive, active_held, held_buttons, released);
    }
}

fn update_trigger(
    button: HandheldButton,
    value: i32,
    active_held: &mut HashSet<HandheldButton>,
    held_buttons: &mut HashMap<HandheldButton, u32>,
    pressed: &mut Vec<HandheldButton>,
    released: &mut Vec<HandheldButton>,
) {
    if value > 0 {
        press_button(button, active_held, held_buttons, pressed);
    } else {
        release_button(button, active_held, held_buttons, released);
    }
}

fn press_button(
    button: HandheldButton,
    active_held: &mut HashSet<HandheldButton>,
    held_buttons: &mut HashMap<HandheldButton, u32>,
    pressed: &mut Vec<HandheldButton>,
) {
    if active_held.insert(button) {
        held_buttons.insert(button, 0);
        pressed.push(button);
    }
}

fn release_button(
    button: HandheldButton,
    active_held: &mut HashSet<HandheldButton>,
    held_buttons: &mut HashMap<HandheldButton, u32>,
    released: &mut Vec<HandheldButton>,
) {
    if active_held.remove(&button) {
        held_buttons.remove(&button);
        released.push(button);
    }
}

pub fn map_evdev_code_to_button(code: u16) -> Option<HandheldButton> {
    match code {
        // D-Pad / Arrows
        103 | 17 => Some(HandheldButton::DpadUp), // KEY_UP / KEY_W
        108 | 31 => Some(HandheldButton::DpadDown), // KEY_DOWN / KEY_S
        105 | 30 => Some(HandheldButton::DpadLeft), // KEY_LEFT / KEY_A
        106 | 32 => Some(HandheldButton::DpadRight), // KEY_RIGHT / KEY_D

        // TrimUI uses a Nintendo-style physical layout: BTN_EAST is the A
        // button and BTN_SOUTH is B (X/Y follow the same physical convention).
        57 | 29 | 305 => Some(HandheldButton::ButtonA), // KEY_SPACE, KEY_LEFTCTRL, BTN_EAST
        56 | 48 | 304 => Some(HandheldButton::ButtonB), // KEY_LEFTALT, KEY_B, BTN_SOUTH
        42 | 45 | 308 => Some(HandheldButton::ButtonX), // KEY_LEFTSHIFT, KEY_X, BTN_WEST
        125 | 21 | 307 | 306 => Some(HandheldButton::ButtonY), // KEY_LEFTMETA, KEY_Y, BTN_NORTH, BTN_C

        // Shoulder Buttons (L1, R1, L2, R2)
        15 | 18 | 310 => Some(HandheldButton::L1), // KEY_TAB, KEY_E, BTN_TL
        14 | 20 | 311 => Some(HandheldButton::R1), // KEY_BACKSPACE, KEY_T, BTN_TR
        104 | 2 | 312 => Some(HandheldButton::L2), // KEY_PAGEUP, KEY_1, BTN_TL2
        109 | 4 | 313 => Some(HandheldButton::R2), // KEY_PAGEDOWN, KEY_3, BTN_TR2

        // System & Menu Buttons
        1 | 172 | 316 | 50 => Some(HandheldButton::Menu), // KEY_ESC, KEY_HOMEPAGE, BTN_MODE, KEY_M
        97 | 314 => Some(HandheldButton::Select),         // KEY_RIGHTCTRL, BTN_SELECT
        28 | 315 => Some(HandheldButton::Start),          // KEY_ENTER, BTN_START
        59 | 87 | 464 | 33 => Some(HandheldButton::FnKey), // KEY_F1, KEY_F11, KEY_FN, KEY_F

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_state() -> (
        HashSet<HandheldButton>,
        HashMap<HandheldButton, u32>,
        Vec<HandheldButton>,
        Vec<HandheldButton>,
    ) {
        (HashSet::new(), HashMap::new(), Vec::new(), Vec::new())
    }

    #[test]
    fn trimui_face_buttons_follow_physical_labels() {
        assert_eq!(map_evdev_code_to_button(305), Some(HandheldButton::ButtonA));
        assert_eq!(map_evdev_code_to_button(304), Some(HandheldButton::ButtonB));
        assert_eq!(map_evdev_code_to_button(308), Some(HandheldButton::ButtonX));
        assert_eq!(map_evdev_code_to_button(307), Some(HandheldButton::ButtonY));
    }

    #[test]
    fn trimui_select_and_start_buttons_are_available_for_chapter_picker() {
        assert_eq!(map_evdev_code_to_button(314), Some(HandheldButton::Select));
        assert_eq!(map_evdev_code_to_button(315), Some(HandheldButton::Start));
    }

    #[test]
    fn dpad_hat_press_direction_change_and_release() {
        let (mut active, mut held, mut pressed, mut released) = empty_state();

        process_event(
            EV_ABS,
            ABS_HAT0X,
            -1,
            &mut active,
            &mut held,
            &mut pressed,
            &mut released,
        );
        assert_eq!(pressed, vec![HandheldButton::DpadLeft]);
        assert!(active.contains(&HandheldButton::DpadLeft));

        pressed.clear();
        process_event(
            EV_ABS,
            ABS_HAT0X,
            1,
            &mut active,
            &mut held,
            &mut pressed,
            &mut released,
        );
        assert_eq!(released, vec![HandheldButton::DpadLeft]);
        assert_eq!(pressed, vec![HandheldButton::DpadRight]);

        released.clear();
        process_event(
            EV_ABS,
            ABS_HAT0X,
            0,
            &mut active,
            &mut held,
            &mut pressed,
            &mut released,
        );
        assert_eq!(released, vec![HandheldButton::DpadRight]);
        assert!(active.is_empty());
        assert!(held.is_empty());
    }

    #[test]
    fn analog_trigger_is_reported_as_button() {
        let (mut active, mut held, mut pressed, mut released) = empty_state();

        process_event(
            EV_ABS,
            ABS_Z,
            255,
            &mut active,
            &mut held,
            &mut pressed,
            &mut released,
        );
        assert_eq!(pressed, vec![HandheldButton::L2]);
        assert!(active.contains(&HandheldButton::L2));

        process_event(
            EV_ABS,
            ABS_Z,
            0,
            &mut active,
            &mut held,
            &mut pressed,
            &mut released,
        );
        assert_eq!(released, vec![HandheldButton::L2]);
        assert!(!active.contains(&HandheldButton::L2));
    }
}
