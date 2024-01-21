use std::collections::HashSet;
use std::hash::Hash;
use winit::keyboard::KeyCode;
use crate::{Plugin, AppBuilder};

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        builder.game().add(Keyboard::new());
    }
}

pub struct Keyboard {
    keys: ButtonState<KeyCode>,
}

impl Keyboard {

    pub fn new() -> Self {
        Self {
            keys: ButtonState::new(),
        }
    }

    /**
     * True if a button is pressed.
    */
    pub fn is_pressed(&mut self, key: KeyCode) -> bool {
        self.keys.is_pressed(key)
    }

    /**
     * True if a button is pressed, but wasn't in the previous tick.
    */
    pub fn is_just_pressed(&mut self, key: KeyCode) -> bool {
        self.keys.is_just_pressed(key)
    }

    /**
     * True if a button is not pressed, but wasn in the previous tick.
    */
    pub fn is_just_released(&mut self, key: KeyCode) -> bool {
        self.keys.is_just_released(key)
    }

        /**
     * Simulates a button press.
    */
    pub fn press(&mut self, key: KeyCode) {
        self.keys.press(key);
    }

    /**
     * Simulates a button release.
    */
    pub fn release(&mut self, key: KeyCode) {
        self.keys.release(key);
    }

    /**
     * Sync previous button state with current button state.
    */
    pub fn sync_previous_state(&mut self) {
        self.keys.sync_previous_state()
    }
}

/**
 * The state of a collection of buttons coming from some device.
 * For example, the buttons on a controller, or the keys on a keyboard.
 */
pub struct ButtonState<B> {
    previous_state: HashSet<B>,
    current_state: HashSet<B>,
}

impl<B> ButtonState<B>
where
    B: Copy + Clone + Eq + Hash
{
    pub fn new() -> Self {
        Self {
            previous_state: HashSet::new(),
            current_state: HashSet::new()
        }
    }

    /**
     * Simulates a button press.
    */
    pub fn press(&mut self, button: B) {
        self.current_state.insert(button);
    }

    /**
     * Simulates a button release.
    */
    pub fn release(&mut self, button: B) {
        self.current_state.remove(&button);
    }

    /**
     * True if a button is pressed.
    */
    pub fn is_pressed(&mut self, button: B) -> bool {
        self.current_state.contains(&button)
    }

    /**
     * True if a button is pressed, but wasn't in the previous tick.
    */
    pub fn is_just_pressed(&mut self, button: B) -> bool {
        self.current_state.contains(&button) && !self.previous_state.contains(&button)
    }

    /**
     * True if a button is not pressed, but wasn in the previous tick.
    */
    pub fn is_just_released(&mut self, button: B) -> bool {
        !self.current_state.contains(&button) && self.previous_state.contains(&button)
    }

    /**
     * Sync previous button state with current button state.
    */
    pub fn sync_previous_state(&mut self) {
        self.previous_state.clear();
        for button in &self.current_state {
            self.previous_state.insert(*button);
        }
    }
}
