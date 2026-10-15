use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use umr_core::{DiagnosticError, ErrorCode, InputEvent, KeyAction, NavigationButton, PointerAction, UmrResult};

/// Active pointer touch state for visual feedback in the display pipeline.
#[derive(Debug, Clone, Copy)]
pub struct ActiveTouchState {
    pub px: u32,
    pub py: u32,
    pub action: PointerAction,
    pub received_at: Instant,
}

/// Android Input Subsystem managing event translation and logcat dispatch.
pub struct AndroidInputSubsystem {
    display_width: u32,
    display_height: u32,
    logcat_path: PathBuf,
    latest_touch: Arc<Mutex<Option<ActiveTouchState>>>,
    latest_nav: Arc<Mutex<Option<(NavigationButton, Instant)>>>,
    event_counter: Arc<Mutex<u64>>,
}

impl AndroidInputSubsystem {
    pub fn new(display_width: u32, display_height: u32, logcat_path: impl AsRef<Path>) -> Self {
        Self {
            display_width,
            display_height,
            logcat_path: logcat_path.as_ref().to_path_buf(),
            latest_touch: Arc::new(Mutex::new(None)),
            latest_nav: Arc::new(Mutex::new(None)),
            event_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Returns the latest active touch state for framebuffer rendering.
    pub fn latest_touch(&self) -> Option<ActiveTouchState> {
        *self.latest_touch.lock().unwrap()
    }

    /// Returns the latest navigation button pressed with timestamp.
    pub fn latest_nav(&self) -> Option<(NavigationButton, Instant)> {
        *self.latest_nav.lock().unwrap()
    }

    /// Clears expired touch state (e.g. touch older than 800ms).
    pub fn clear_expired_touch(&self, max_age_ms: u128) {
        let mut touch_guard = self.latest_touch.lock().unwrap();
        if let Some(touch) = *touch_guard {
            if touch.received_at.elapsed().as_millis() > max_age_ms {
                *touch_guard = None;
            }
        }
    }

    /// Dispatches an abstract input event, translating to Android motion/key events.
    pub fn dispatch(&mut self, event: InputEvent) -> UmrResult<()> {
        event.validate().map_err(|msg| {
            DiagnosticError::new(ErrorCode::ErrLifecycleInvalidTransition, msg)
        })?;

        let mut counter = self.event_counter.lock().unwrap();
        *counter += 1;
        let event_id = *counter;

        match event {
            InputEvent::Pointer { x, y, action } => {
                let px = ((x.clamp(0.0, 1.0)) * (self.display_width as f32 - 1.0)).round() as u32;
                let py = ((y.clamp(0.0, 1.0)) * (self.display_height as f32 - 1.0)).round() as u32;

                let action_str = match action {
                    PointerAction::Down => "ACTION_DOWN",
                    PointerAction::Move => "ACTION_MOVE",
                    PointerAction::Up => "ACTION_UP",
                    PointerAction::Cancel => "ACTION_CANCEL",
                };

                // Update active touch state
                let mut touch_guard = self.latest_touch.lock().unwrap();
                if action == PointerAction::Up || action == PointerAction::Cancel {
                    *touch_guard = None;
                } else {
                    *touch_guard = Some(ActiveTouchState {
                        px,
                        py,
                        action,
                        received_at: Instant::now(),
                    });
                }

                // Log to logcat
                self.append_log(&format!(
                    "D/InputReader: [#{}] MotionEvent {{ action={}, x={}, y={}, pressure=1.0, pointerCount=1 }}",
                    event_id, action_str, px, py
                ))?;
                self.append_log(&format!(
                    "D/InputDispatcher: [#{}] Delivering touch to focused window 'com.umr.surface.0'",
                    event_id
                ))?;
            }

            InputEvent::Key { key_code, action } => {
                let action_str = match action {
                    KeyAction::Down => "ACTION_DOWN",
                    KeyAction::Up => "ACTION_UP",
                };

                self.append_log(&format!(
                    "D/InputReader: [#{}] KeyEvent {{ action={}, keyCode={} }}",
                    event_id, action_str, key_code
                ))?;
                self.append_log(&format!(
                    "D/InputDispatcher: [#{}] Delivering key code {} to focused activity",
                    event_id, key_code
                ))?;
            }

            InputEvent::Navigation { button } => {
                let key_code = button.to_android_keycode();
                let button_name = match button {
                    NavigationButton::Back => "BACK",
                    NavigationButton::Home => "HOME",
                    NavigationButton::Recents => "APP_SWITCH (Recents)",
                };

                *self.latest_nav.lock().unwrap() = Some((button, Instant::now()));

                self.append_log(&format!(
                    "D/InputReader: [#{}] Hardware navigation button pressed: {} (keyCode={})",
                    event_id, button_name, key_code
                ))?;
                self.append_log(&format!(
                    "D/PhoneWindowManager: Intercepting navigation key {} -> executing action",
                    key_code
                ))?;
            }
        }

        Ok(())
    }

    fn append_log(&self, line: &str) -> UmrResult<()> {
        if !self.logcat_path.exists() {
            return Ok(());
        }
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.logcat_path)
            .map_err(|e| {
                DiagnosticError::new(
                    ErrorCode::ErrBackendInternal,
                    format!("Failed to write to logcat: {}", e),
                )
            })?;
        writeln!(file, "{}", line).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendInternal,
                format!("Failed to write to logcat: {}", e),
            )
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_input_pointer_translation() {
        let temp = tempdir().unwrap();
        let logcat_path = temp.path().join("logcat.log");
        std::fs::write(&logcat_path, "header\n").unwrap();

        let mut subsystem = AndroidInputSubsystem::new(360, 640, &logcat_path);

        // Send pointer down at center (0.5, 0.5)
        let ev = InputEvent::pointer(0.5, 0.5, PointerAction::Down);
        subsystem.dispatch(ev).expect("dispatch pointer");

        let touch = subsystem.latest_touch().expect("touch should be registered");
        // (0.5 * 359) round = 180, (0.5 * 639) round = 320
        assert_eq!(touch.px, 180);
        assert_eq!(touch.py, 320);
        assert_eq!(touch.action, PointerAction::Down);

        let logs = std::fs::read_to_string(&logcat_path).unwrap();
        assert!(logs.contains("MotionEvent { action=ACTION_DOWN, x=180, y=320"));

        // Pointer up clears latest touch
        let up_ev = InputEvent::pointer(0.5, 0.5, PointerAction::Up);
        subsystem.dispatch(up_ev).expect("dispatch up");
        assert!(subsystem.latest_touch().is_none());
    }

    #[test]
    fn test_input_navigation_key() {
        let temp = tempdir().unwrap();
        let logcat_path = temp.path().join("logcat.log");
        std::fs::write(&logcat_path, "header\n").unwrap();

        let mut subsystem = AndroidInputSubsystem::new(360, 640, &logcat_path);
        subsystem
            .dispatch(InputEvent::navigation(NavigationButton::Back))
            .unwrap();

        let logs = std::fs::read_to_string(&logcat_path).unwrap();
        assert!(logs.contains("Hardware navigation button pressed: BACK (keyCode=4)"));
    }
}

