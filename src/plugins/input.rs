use std::collections::VecDeque;
use std::hash::Hash;
use glam::Vec2;
use winit::keyboard::KeyCode;
use crate::{AppBuilder, Game, GraphicsState, HashSet, Plugin, RunContext, Stage};

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn install(&mut self, builder: &mut AppBuilder) {
        builder.game()
            .add(InputRequests::default())
            .add(Keyboard::default())
            .add(Cursor::default());
        builder.system(Stage::SyncInput, sync_inputs);
    }
}

#[derive(Default)]
pub struct Keyboard {
    keys: ButtonState<KeyCode>,
}

pub struct Cursor {
    pub(crate) position: Vec2,
    pub(crate) movement: Vec2,
    pub(crate) is_grabbed: bool,
    pub(crate) is_visible: bool,
}

impl Cursor {

    pub fn position(&self) -> Vec2 {
        self.position
    }

    // Movement of the cursor since the last tick.
    pub fn movement(&self) -> Vec2 {
        self.movement
    }

    pub fn is_grabbed(&self) -> bool {
        self.is_grabbed
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            movement: Vec2::ZERO,
            is_grabbed: false,
            is_visible: true,
        }
    }
}

impl Keyboard {

    /**
     * True if a button is pressed.
    */
    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.keys.is_pressed(key)
    }

    /**
     * True if a button is pressed, but wasn't in the previous tick.
    */
    pub fn is_just_pressed(&self, key: KeyCode) -> bool {
        self.keys.is_just_pressed(key)
    }

    /**
     * True if a button is not pressed, but wasn in the previous tick.
    */
    pub fn is_just_released(&self, key: KeyCode) -> bool {
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

impl<B> Default for ButtonState<B> {
    fn default() -> Self {
        Self {
            previous_state: HashSet::default(),
            current_state: HashSet::default(),
        }
    }
}

impl<B> ButtonState<B>
where
    B: Copy + Clone + Eq + Hash
{
    pub fn new() -> Self {
        Self {
            previous_state: HashSet::default(),
            current_state: HashSet::default()
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
    pub fn is_pressed(&self, button: B) -> bool {
        self.current_state.contains(&button)
    }

    /**
     * True if a button is pressed, but wasn't in the previous tick.
    */
    pub fn is_just_pressed(&self, button: B) -> bool {
        self.current_state.contains(&button) && !self.previous_state.contains(&button)
    }

    /**
     * True if a button is not pressed, but wasn in the previous tick.
    */
    pub fn is_just_released(&self, button: B) -> bool {
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


fn sync_inputs(game: &mut Game, _ctx: RunContext) {
    let mut keyboard = game.get::<&mut Keyboard>();
    let mut cursor = game.get::<&mut Cursor>();
    let mut requests = game.get::<&mut InputRequests>();

    keyboard.sync_previous_state();
    cursor.movement = Vec2::ZERO;
    if cursor.is_grabbed {
        let state = game.get::<&mut GraphicsState>();
        let center = state.center();
        requests.push(InputRequest::SetCursorPosition(center));
    }
}

/// Queue of requests to dispatch to the application's runner.
#[derive(Default)]
pub struct InputRequests(VecDeque<InputRequest>);
impl InputRequests {

    pub fn set_cursor_position(&mut self, position: Vec2) {
        self.push(InputRequest::SetCursorPosition(position));
    }

    pub fn hide_cursor(&mut self) {
        self.push(InputRequest::HideCursor);
    }

    pub fn show_cursor(&mut self) {
        self.push(InputRequest::ShowCursor);
    }

    pub fn grab_cursor(&mut self) {
        self.push(InputRequest::GrabCursor);
    }

    pub fn ungrab_cursor(&mut self) {
        self.push(InputRequest::UngrabCursor);
    }

    pub fn push(&mut self, request: InputRequest) {
        self.0.push_back(request);
    }

    pub(crate) fn pop(&mut self) -> Option<InputRequest> {
        self.0.pop_front()
    }
}


/// Request that application code makes to the window manager.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum InputRequest {
    SetCursorPosition(Vec2),
    HideCursor,
    ShowCursor,
    GrabCursor,
    UngrabCursor,
}