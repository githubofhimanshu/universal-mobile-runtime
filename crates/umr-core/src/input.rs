use serde::{Deserialize, Serialize};

/// Action performed in a pointer or touch event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerAction {
    Down,
    Move,
    Up,
    Cancel,
}

impl std::fmt::Display for PointerAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Down => write!(f, "down"),
            Self::Move => write!(f, "move"),
            Self::Up => write!(f, "up"),
            Self::Cancel => write!(f, "cancel"),
        }
    }
}

/// Action performed in a hardware or virtual key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyAction {
    Down,
    Up,
}

impl std::fmt::Display for KeyAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Down => write!(f, "down"),
            Self::Up => write!(f, "up"),
        }
    }
}

/// Hardware navigation buttons typical of mobile host devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationButton {
    Back,
    Home,
    Recents,
}

/// Canonical Android Keycode Constants
pub const KEYCODE_UNKNOWN: u32 = 0;
pub const KEYCODE_HOME: u32 = 3;
pub const KEYCODE_BACK: u32 = 4;
pub const KEYCODE_VOLUME_UP: u32 = 24;
pub const KEYCODE_VOLUME_DOWN: u32 = 25;
pub const KEYCODE_POWER: u32 = 26;
pub const KEYCODE_APP_SWITCH: u32 = 187; // Recents button

impl NavigationButton {
    /// Translates canonical mobile navigation buttons to standard Android keycodes.
    pub fn to_android_keycode(&self) -> u32 {
        match self {
            Self::Back => KEYCODE_BACK,
            Self::Home => KEYCODE_HOME,
            Self::Recents => KEYCODE_APP_SWITCH,
        }
    }
}

/// Typed mobile input event emitted from a Desktop Shell or client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputEvent {
    /// Pointer or touch event with normalized coordinates in range [0.0, 1.0].
    Pointer {
        x: f32,
        y: f32,
        action: PointerAction,
    },
    /// Physical or virtual keyboard event with keycode and action.
    Key {
        key_code: u32,
        action: KeyAction,
    },
    /// Hardware navigation button action (Back, Home, Recents).
    Navigation {
        button: NavigationButton,
    },
}

impl InputEvent {
    /// Creates a pointer touch event. Coordinates are normalized [0.0, 1.0].
    pub fn pointer(x: f32, y: f32, action: PointerAction) -> Self {
        Self::Pointer { x, y, action }
    }

    /// Creates a key event.
    pub fn key(key_code: u32, action: KeyAction) -> Self {
        Self::Key { key_code, action }
    }

    /// Creates a navigation button event.
    pub fn navigation(button: NavigationButton) -> Self {
        Self::Navigation { button }
    }

    /// Validates coordinate boundaries and event integrity.
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Pointer { x, y, .. } => {
                if !x.is_finite() || !y.is_finite() {
                    return Err("Pointer coordinates must be finite numbers");
                }
                if *x < 0.0 || *x > 1.0 || *y < 0.0 || *y > 1.0 {
                    return Err("Normalized pointer coordinates must be within [0.0, 1.0]");
                }
                Ok(())
            }
            Self::Key { key_code, .. } => {
                if *key_code == KEYCODE_UNKNOWN {
                    return Err("Key code cannot be KEYCODE_UNKNOWN (0)");
                }
                Ok(())
            }
            Self::Navigation { .. } => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointer_validation() {
        let valid = InputEvent::pointer(0.5, 0.5, PointerAction::Down);
        assert!(valid.validate().is_ok());

        let out_of_bounds = InputEvent::pointer(1.2, 0.5, PointerAction::Down);
        assert!(out_of_bounds.validate().is_err());

        let nan_coord = InputEvent::pointer(f32::NAN, 0.5, PointerAction::Down);
        assert!(nan_coord.validate().is_err());
    }

    #[test]
    fn test_navigation_button_mapping() {
        assert_eq!(NavigationButton::Back.to_android_keycode(), KEYCODE_BACK);
        assert_eq!(NavigationButton::Home.to_android_keycode(), KEYCODE_HOME);
        assert_eq!(NavigationButton::Recents.to_android_keycode(), KEYCODE_APP_SWITCH);
    }

    #[test]
    fn test_input_serialization() {
        let event = InputEvent::pointer(0.25, 0.75, PointerAction::Move);
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("\"type\":\"pointer\""));
        assert!(serialized.contains("\"action\":\"move\""));

        let deserialized: InputEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(event, deserialized);
    }
}

