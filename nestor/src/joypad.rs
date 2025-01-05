use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    // https://wiki.nesdev.com/w/index.php/Controller_reading_code
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[repr(transparent)]
    pub struct JoypadButton: u8 {
        const RIGHT             = 0b10000000;
        const LEFT              = 0b01000000;
        const DOWN              = 0b00100000;
        const UP                = 0b00010000;
        const START             = 0b00001000;
        const SELECT            = 0b00000100;
        const BUTTON_B          = 0b00000010;
        const BUTTON_A          = 0b00000001;
    }
}

#[derive(Clone)]
pub struct Joypad {
    strobe: bool,
    button_index: u8,
    button_status: JoypadButton,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            strobe: false,
            button_index: 0,
            button_status: JoypadButton::from_bits_truncate(0),
        }
    }

    pub fn write(&mut self, data: u8) {
        let new_strobe = data & 1 == 1;

        if self.strobe && !new_strobe {
            // Strobe transitioned from 1 to 0, reset button_index
            self.button_index = 0;
        }

        self.strobe = new_strobe;
    }

    pub fn read(&mut self) -> u8 {
        if self.button_index > 7 {
            return 1;
        }
        let response = (self.button_status.bits() & (1 << self.button_index)) >> self.button_index;
        if !self.strobe && self.button_index <= 7 {
            self.button_index += 1;
        }
        response
    }

    pub fn set_button_pressed_status(&mut self, button: JoypadButton, pressed: bool) {
        self.button_status.set(button, pressed);
    }
}

impl Default for Joypad {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_button_pressed_status() {
        for button in [
            JoypadButton::BUTTON_A,
            JoypadButton::BUTTON_B,
            JoypadButton::START,
            JoypadButton::SELECT,
            JoypadButton::UP,
            JoypadButton::DOWN,
            JoypadButton::LEFT,
            JoypadButton::RIGHT,
        ] {
            let mut joypad = Joypad::new();

            joypad.set_button_pressed_status(button.clone(), true);

            assert!(joypad.button_status.contains(button))
        }
    }

    #[test]
    fn test_write_strobe() {
        let mut joypad = Joypad::new();

        assert!(!joypad.strobe);

        // Write 1 to strobe
        joypad.write(1);
        assert!(joypad.strobe);
        assert_eq!(joypad.button_index, 0);

        // Write 0 to strobe
        joypad.write(0);
        assert!(!joypad.strobe);
        // button_index remains the same unless read
    }

    #[test]
    fn test_read_no_buttons_pressed() {
        let mut joypad = Joypad::new();

        // Read all button states
        for _ in 0..8 {
            let value = joypad.read();
            assert_eq!(value, 0);
        }

        // After 8 reads, further reads should return 1
        assert_eq!(joypad.read(), 1);
    }

    #[test]
    fn test_read_single_button_pressed() {
        let mut joypad = Joypad::new();

        joypad.set_button_pressed_status(JoypadButton::BUTTON_A, true);

        for i in 0..8 {
            let value = joypad.read();
            if i == 0 {
                // BUTTON_A is the first button
                assert_eq!(value, 1);
            } else {
                assert_eq!(value, 0);
            }
        }

        assert_eq!(joypad.read(), 1);
    }

    #[test]
    fn test_read_multiple_buttons_pressed() {
        let mut joypad = Joypad::new();

        // Press A and START buttons
        joypad.set_button_pressed_status(JoypadButton::BUTTON_A, true);
        joypad.set_button_pressed_status(JoypadButton::START, true);

        // Expected button states in order: A, B, SELECT, START, UP, DOWN, LEFT, RIGHT
        let expected_values = [1, 0, 0, 1, 0, 0, 0, 0];

        for &expected in &expected_values {
            let value = joypad.read();
            assert_eq!(value, expected);
        }

        // After 8 reads, further reads should return 1
        assert_eq!(joypad.read(), 1);
    }

    #[test]
    fn test_strobe_high_behavior() {
        let mut joypad = Joypad::new();

        // Press B and UP buttons
        joypad.set_button_pressed_status(JoypadButton::BUTTON_B, true);
        joypad.set_button_pressed_status(JoypadButton::UP, true);

        // Write 1 to strobe (strobe high)
        joypad.write(1);

        // Read multiple times while strobe is high
        for _ in 0..10 {
            let value = joypad.read();
            // When strobe is high, the joypad should repeatedly return the state of the first button
            // BUTTON_A is the first button, which is not pressed in this test
            assert_eq!(value, 0);
        }

        // Now press BUTTON_A
        joypad.set_button_pressed_status(JoypadButton::BUTTON_A, true);

        // Read again while strobe is still high
        let value = joypad.read();
        assert_eq!(value, 1);

        // Write 0 to strobe to reset
        joypad.write(0);

        // Read the button states sequentially
        let expected_values = [1, 1, 0, 0, 1, 0, 0, 0]; // BUTTON_A, BUTTON_B, SELECT, START, UP, DOWN, LEFT, RIGHT

        for &expected in &expected_values {
            let value = joypad.read();
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn test_read_after_full_cycle() {
        let mut joypad = Joypad::new();

        // Press LEFT and DOWN buttons
        joypad.set_button_pressed_status(JoypadButton::LEFT, true);
        joypad.set_button_pressed_status(JoypadButton::DOWN, true);

        // Write 0 to strobe to reset (ensure previous strobe was 1)
        joypad.write(1);
        joypad.write(0);

        // Read all 8 button states
        for _ in 0..8 {
            joypad.read();
        }

        // After 8 reads, further reads should return 1
        for _ in 0..5 {
            let value = joypad.read();
            assert_eq!(value, 1);
        }

        // Reset strobe to read again
        joypad.write(1);
        joypad.write(0);

        // Read the button states again
        let expected_values = [0, 0, 0, 0, 0, 1, 1, 0]; // BUTTON_A to RIGHT

        for &expected in &expected_values {
            let value = joypad.read();
            assert_eq!(value, expected);
        }
    }

    #[test]
    fn test_toggle_buttons_during_read() {
        let mut joypad = Joypad::new();

        // Initially, no buttons pressed
        joypad.write(0);

        // Read BUTTON_A
        assert_eq!(joypad.read(), 0);

        // Press BUTTON_B
        joypad.set_button_pressed_status(JoypadButton::BUTTON_B, true);

        // Read BUTTON_B
        assert_eq!(joypad.read(), 1);

        // Release BUTTON_B and press START
        joypad.set_button_pressed_status(JoypadButton::BUTTON_B, false);
        joypad.set_button_pressed_status(JoypadButton::START, true);

        // Read SELECT
        assert_eq!(joypad.read(), 0);

        // Read START
        assert_eq!(joypad.read(), 1);
    }

    #[test]
    fn test_strobe_transition_behavior() {
        let mut joypad = Joypad::new();

        assert!(!joypad.strobe);
        assert_eq!(joypad.button_index, 0);

        joypad.write(0); // Write 0 to strobe (no transition, strobe remains false)
        assert!(!joypad.strobe);

        joypad.read(); // Perform a read to increment button_index
        assert_eq!(joypad.button_index, 1);

        joypad.write(1); // Write 1 to strobe (transition from 0 to 1)
        joypad.read();

        assert!(joypad.strobe);
        assert_eq!(joypad.button_index, 1); // button_index is not reseted

        joypad.write(0);
        assert!(!joypad.strobe);
        assert_eq!(joypad.button_index, 0); // button_index resets on transition from 1 to 0
    }
}
