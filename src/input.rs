use minifb::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandheldButton {
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    ButtonA,
    ButtonB,
    ButtonX,
    ButtonY,
    L1,
    R1,
    L2,
    R2,
    Menu,
    Select,
    Start,
    FnKey,
}

#[derive(Debug, Default, Clone)]
pub struct InputState {
    pub keys_pressed: Vec<HandheldButton>,
    pub keys_held: Vec<HandheldButton>,
    pub keys_released: Vec<HandheldButton>,
}

impl InputState {
    pub fn is_pressed(&self, button: HandheldButton) -> bool {
        self.keys_pressed.contains(&button)
    }

    pub fn is_held(&self, button: HandheldButton) -> bool {
        self.keys_held.contains(&button)
    }

    pub fn is_released(&self, button: HandheldButton) -> bool {
        self.keys_released.contains(&button)
    }
}

pub struct InputManager {
    prev_keys: Vec<Key>,
    held_frames: std::collections::HashMap<Key, u32>,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            prev_keys: Vec::new(),
            held_frames: std::collections::HashMap::new(),
        }
    }

    pub fn update(&mut self, current_keys: &[Key]) -> InputState {
        let mut pressed = Vec::new();
        let mut held = Vec::new();
        let mut released = Vec::new();

        for key in current_keys {
            if let Some(btn) = Self::map_key_to_button(*key) {
                if !self.prev_keys.contains(key) {
                    pressed.push(btn);
                    self.held_frames.insert(*key, 0);
                } else {
                    let frames = self.held_frames.entry(*key).or_insert(0);
                    *frames += 1;
                    held.push(btn);

                    // Key repeat acceleration for DPad navigation/pan
                    if *frames > 15 && (*frames % 4 == 0) {
                        pressed.push(btn);
                    }
                }
            }
        }

        for key in &self.prev_keys {
            if !current_keys.contains(key) {
                if let Some(btn) = Self::map_key_to_button(*key) {
                    released.push(btn);
                }
                self.held_frames.remove(key);
            }
        }

        self.prev_keys = current_keys.to_vec();

        InputState {
            keys_pressed: pressed,
            keys_held: held,
            keys_released: released,
        }
    }

    fn map_key_to_button(key: Key) -> Option<HandheldButton> {
        match key {
            Key::Up | Key::W => Some(HandheldButton::DpadUp),
            Key::Down | Key::S => Some(HandheldButton::DpadDown),
            Key::Left | Key::A => Some(HandheldButton::DpadLeft),
            Key::Right | Key::D => Some(HandheldButton::DpadRight),

            Key::J | Key::Space => Some(HandheldButton::ButtonA), // Confirm / Next page
            Key::K | Key::Backspace => Some(HandheldButton::ButtonB), // Back / Cancel
            Key::U | Key::X => Some(HandheldButton::ButtonX), // Reset zoom / Extra action
            Key::I | Key::Y => Some(HandheldButton::ButtonY), // Theme / RTL switch

            Key::Q | Key::PageUp => Some(HandheldButton::L1), // Prev page
            Key::E | Key::PageDown => Some(HandheldButton::R1), // Next page
            Key::Key1 | Key::NumPad1 | Key::LeftBracket => Some(HandheldButton::L2), // Zoom out / Fast back
            Key::Key3 | Key::NumPad3 | Key::RightBracket => Some(HandheldButton::R2), // Zoom in / Fast fwd

            Key::Escape | Key::M => Some(HandheldButton::Menu),
            Key::Tab => Some(HandheldButton::Select),
            Key::Enter => Some(HandheldButton::Start),
            Key::F | Key::F1 => Some(HandheldButton::FnKey),

            _ => None,
        }
    }
}
