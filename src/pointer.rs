use crate::Modifiers;
use crate::kurbo::{Point, Size, Vec2};

#[derive(Debug, Clone, PartialEq)]
pub struct PenInfo {
    pub pressure: f32,
    pub tangential_pressure: f32,
    pub twist: u32,

    // TODO: We should normalise to one or the other of these.  Azimuth/angle seems conceptually easier to work with?
    pub tilt_x: i32,
    pub tilt_y: i32,
    pub azimuth_angle: f64,
    pub altitude_angle: f64
}

#[derive(Debug, Clone, PartialEq)]
pub struct TouchInfo {
    pub contact_geometry: Size,
    pub pressure: Option<f32>,
    // TODO: Phase?
}

#[derive(Debug, Clone, PartialEq)]
pub struct MouseInfo {
    wheel_delta: Vec2,
}

impl Default for PenInfo {
    fn default() -> Self {
        PenInfo {
            pressure: 0.0, // In the range zero to one, must be 0.5 when in active buttons state for hardware that doesn't support pressure, and 0 otherwise
            tangential_pressure: 0.0,
            tilt_x: 0,
            tilt_y: 0,
            twist: 0,
            azimuth_angle: 0.0,
            altitude_angle: 0.0,
        }
    }
}

impl Default for TouchInfo {
    fn default() -> Self {
        Self {
            pressure: None, // In the range zero to one, must be 0.5 when in active buttons state for hardware that doesn't support pressure, and 0 otherwise
            contact_geometry: Size::new(1., 1.),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerType {
    Mouse(MouseInfo),
    Pen(PenInfo),
    Touch(TouchInfo),
    // Apple has force touch devices that provide pressure info, but nothing further.
    // Assume that that may become more of a thing in the future?
}

/// An indicator of which pointer button was pressed.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u8)]
pub enum PointerButton {
    /// No mouse button.
    // MUST BE FIRST (== 0)
    None,
    /// Left mouse button, Left Mouse, Touch Contact, Pen contact.
    Left,
    /// Right mouse button, Right Mouse, Pen barrel button.
    Right,
    /// Middle mouse button.
    Middle,
    /// X1 (back) Mouse.
    X1,
    /// X2 (forward) Mouse.
    X2,
    /// Pen eraser button
    Eraser
}


impl PointerButton {
    /// Returns `true` if this is [`PointerButton::Left`].
    ///
    /// [`MouseButton::Left`]: #variant.Left
    #[inline]
    pub fn is_left(self) -> bool {
        self == PointerButton::Left
    }

    /// Returns `true` if this is [`PointerButton::Right`].
    ///
    /// [`PointerButton::Right`]: #variant.Right
    #[inline]
    pub fn is_right(self) -> bool {
        self == PointerButton::Right
    }

    /// Returns `true` if this is [`PointerButton::Middle`].
    ///
    /// [`PointerButton::Middle`]: #variant.Middle
    #[inline]
    pub fn is_middle(self) -> bool {
        self == PointerButton::Middle
    }

    /// Returns `true` if this is [`PointerButton::X1`].
    ///
    /// [`PointerButton::X1`]: #variant.X1
    #[inline]
    pub fn is_x1(self) -> bool {
        self == PointerButton::X1
    }

    /// Returns `true` if this is [`PointerButton::X2`].
    ///
    /// [`PointerButton::X2`]: #variant.X2
    #[inline]
    pub fn is_x2(self) -> bool {
        self == PointerButton::X2
    }

    /// Returns `true` if this is [`PointerButton::Eraser`].
    ///
    /// [`PointerButton::Eraser`]: #variant.X2
    #[inline]
    pub fn is_eraser(self) -> bool {
        self == PointerButton::Eraser
    }
}

/// A set of [`PointerButton`]s.
///
/// [`PointerButton`]: enum.PointerButton.html
#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct PointerButtons(u8);

impl PointerButtons {
    /// Create a new empty set.
    #[inline]
    pub fn new() -> PointerButtons {
        PointerButtons(0)
    }

    /// Add the `button` to the set.
    #[inline]
    pub fn insert(&mut self, button: PointerButton) {
        self.0 |= 1.min(button as u8) << button as u8;
    }

    /// Remove the `button` from the set.
    #[inline]
    pub fn remove(&mut self, button: PointerButton) {
        self.0 &= !(1.min(button as u8) << button as u8);
    }

    /// Builder-style method for adding the `button` to the set.
    #[inline]
    pub fn with(mut self, button: PointerButton) -> PointerButtons {
        self.0 |= 1.min(button as u8) << button as u8;
        self
    }

    /// Builder-style method for removing the `button` from the set.
    #[inline]
    pub fn without(mut self, button: PointerButton) -> PointerButtons {
        self.0 &= !(1.min(button as u8) << button as u8);
        self
    }

    /// Returns `true` if the `button` is in the set.
    #[inline]
    pub fn contains(self, button: PointerButton) -> bool {
        (self.0 & (1.min(button as u8) << button as u8)) != 0
    }

    /// Returns `true` if the set is empty.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if all the `buttons` are in the set.
    #[inline]
    pub fn is_superset(self, buttons: PointerButtons) -> bool {
        self.0 & buttons.0 == buttons.0
    }

    /// Returns `true` if [`PointerButton::Left`] is in the set.
    ///
    /// [`PointerButton::Left`]: enum.PointerButton.html#variant.Left
    #[inline]
    pub fn has_left(self) -> bool {
        self.contains(PointerButton::Left)
    }

    /// Returns `true` if [`PointerButton::Right`] is in the set.
    ///
    /// [`PointerButton::Right`]: enum.PointerButton.html#variant.Right
    #[inline]
    pub fn has_right(self) -> bool {
        self.contains(PointerButton::Right)
    }

    /// Returns `true` if [`PointerButton::Middle`] is in the set.
    ///
    /// [`PointerButton::Middle`]: enum.PointerButton.html#variant.Middle
    #[inline]
    pub fn has_middle(self) -> bool {
        self.contains(PointerButton::Middle)
    }

    /// Returns `true` if [`PointerButton::X1`] is in the set.
    ///
    /// [`PointerButton::X1`]: enum.PointerButton.html#variant.X1
    #[inline]
    pub fn has_x1(self) -> bool {
        self.contains(PointerButton::X1)
    }

    /// Returns `true` if [`PointerButton::X2`] is in the set.
    ///
    /// [`PointerButton::X2`]: enum.PointerButton.html#variant.X2
    #[inline]
    pub fn has_x2(self) -> bool {
        self.contains(PointerButton::X2)
    }

    /// Returns `true` if [`PointerButton::Eraser`] is in the set.
    ///
    /// [`PointerButton::Eraser`]: enum.PointerButton.html#variant.Eraser
    #[inline]
    pub fn has_eraser(self) -> bool {
        self.contains(PointerButton::Eraser)
    }

    /// Adds all the `buttons` to the set.
    pub fn extend(&mut self, buttons: PointerButtons) {
        self.0 |= buttons.0;
    }

    /// Returns a union of the values in `self` and `other`.
    #[inline]
    pub fn union(mut self, other: PointerButtons) -> PointerButtons {
        self.0 |= other.0;
        self
    }

    /// Clear the set.
    #[inline]
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Count the number of pressed buttons in the set.
    #[inline]
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }
}

impl std::fmt::Debug for PointerButtons {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PointerButtons({:05b})", self.0 >> 1)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointerEvent {
    pub timestamp: u64, // Timestamp of the actual event
    pub pos: Point,
    pub buttons: PointerButtons,
    pub modifiers: Modifiers,
    /// The button that was pressed down in the case of mouse-down,
    /// or the button that was released in the case of mouse-up.
    /// This will always be `PointerButton::None` in the case of mouse-move/touch.
    pub button: PointerButton,

    /// Focus is `true` on macOS when the mouse-down event (or its companion mouse-up event)
    /// with `MouseButton::Left` was the event that caused the window to gain focus.
    pub focus: bool,

    // TODO: Should this be here, or only in mouse/pen events?
    pub count: u8,

    // This is a super-set of mouse events and stylus + touch events.
    pub pointer_id: u32,
    pub is_primary: bool,
    pub pointer_type: PointerType,
}

// Do we need a way of getting at maxTouchPoints?

impl Default for PointerEvent {
    fn default() -> Self {
        PointerEvent {
            timestamp: 0,
            pos: Default::default(),
            buttons: Default::default(),
            modifiers: Default::default(),
            button: PointerButton::None,
            focus: false,
            count: 0,
            pointer_id: 0,
            is_primary: true,
            pointer_type: PointerType::Mouse(MouseInfo { wheel_delta: Vec2::ZERO }),
        }
    }
}

impl PointerEvent {
    // TODO - lots of helper functions - is_hovering?

    pub fn is_touch() -> bool {
        todo!();
    }

    pub fn is_mouse() -> bool {
        todo!();
    }

    pub fn is_pen() -> bool {
        todo!();
    }
}
