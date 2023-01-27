use crate::kurbo::{Point, Size, Vec2};
use crate::Modifiers;
use std::time::Instant;
#[derive(Debug, Clone, PartialEq)]
pub struct PenInclinationAzimuthAltitude {
    pub azimuth_angle: f32,
    pub altitude_angle: f32,
}
#[derive(Debug, Clone, PartialEq)]
pub struct PenInclinationTilt {
    pub tilt_x: i32,
    pub tilt_y: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PenInclination {
    Tilt(i32, i32),            // tiltX, tiltY in degrees -90..90
    AzimuthAltitude(f32, f32), // Azimuth angle, altitude angle in radians
}

impl PenInclination {
    // NOTE: We store pen inclination as whatever the platform gives it to us as - either tilt x/y or azimuth_angle/altitude_angle.
    //  It can be requested as either tilt or azimuth/angle form, and conversion is only performed on demand.
    //  Functions are taken from:
    //  https://www.w3.org/TR/pointerevents3/#converting-between-tiltx-tilty-and-altitudeangle-azimuthangle

    pub fn from_tilt(tilt_x: i32, tilt_y: i32) -> PenInclination {
        PenInclination::Tilt(tilt_x, tilt_y)
    }
    pub fn from_angle(azimuth_angle: f32, altitude_angle: f32) -> PenInclination {
        PenInclination::AzimuthAltitude(azimuth_angle, altitude_angle)
    }

    pub fn tilt(&self) -> PenInclinationTilt {
        match *self {
            PenInclination::Tilt(tilt_x, tilt_y) => PenInclinationTilt { tilt_x, tilt_y },
            PenInclination::AzimuthAltitude(azimuth_angle, altitude_angle) => {
                PenInclination::spherical_to_tilt(azimuth_angle, altitude_angle)
            }
        }
    }

    pub fn azimuth_altitude(&self) -> PenInclinationAzimuthAltitude {
        match *self {
            PenInclination::Tilt(tilt_x, tilt_y) => {
                PenInclination::tilt_to_spherical(tilt_x, tilt_y)
            }
            PenInclination::AzimuthAltitude(azimuth_angle, altitude_angle) => {
                PenInclinationAzimuthAltitude {
                    azimuth_angle,
                    altitude_angle,
                }
            }
        }
    }

    fn tilt_to_spherical(tilt_x: i32, tilt_y: i32) -> PenInclinationAzimuthAltitude {
        use std::f32::consts::{PI, TAU};
        let tilt_x_rad = tilt_x as f32 * PI / 180.0;
        let tilt_y_rad = tilt_y as f32 * PI / 180.0;

        // calculate azimuth angle
        let mut azimuth_angle = 0.0;

        if tilt_x == 0 {
            if tilt_y > 0 {
                azimuth_angle = PI / 2.0;
            } else if tilt_y < 0 {
                azimuth_angle = 3.0 * PI / 2.0;
            }
        } else if tilt_y == 0 {
            if tilt_x < 0 {
                azimuth_angle = PI;
            }
        } else if tilt_x.abs() == 90 || tilt_y.abs() == 90 {
            // not enough information to calculate azimuth
            azimuth_angle = 0.0;
        } else {
            // Non-boundary case: neither tiltX nor tiltY is equal to 0 or +-90
            let tan_x = tilt_x_rad.tan();
            let tan_y = tilt_x_rad.tan();

            azimuth_angle = f32::atan2(tan_y, tan_x);
            if azimuth_angle < 0.0 {
                azimuth_angle += TAU;
            }
        }

        // calculate altitude angle
        let altitude_angle = if tilt_x.abs() == 90 || tilt_y.abs() == 90 {
            0.0
        } else if tilt_x == 0 {
            PI / 2.0 - tilt_y_rad.abs()
        } else if tilt_y == 0 {
            PI / 2.0 - tilt_x_rad.abs()
        } else {
            // Non-boundary case: neither tiltX nor tiltY is equal to 0 or +-90
            let tan_x = tilt_x_rad.tan();
            let tan_y = tilt_x_rad.tan();
            f32::atan(1.0 / (tan_x * tan_x + tan_y * tan_y).sqrt())
        };
        PenInclinationAzimuthAltitude {
            altitude_angle,
            azimuth_angle,
        }
    }

    fn spherical_to_tilt(altitude_angle: f32, azimuth_angle: f32) -> PenInclinationTilt {
        use std::f32::consts::{PI, TAU};
        let rad_to_deg = 180.0 / PI;
        let mut tilt_y_rad = 0.0;
        let mut tilt_x_rad = 0.0;
        if altitude_angle == 0.0 {
            // the pen is in the X-Y plane
            if azimuth_angle == 0.0 || azimuth_angle == TAU {
                // pen is on positive X axis
                tilt_x_rad = PI / 2.0;
            }
            if azimuth_angle == PI / 2.0 {
                // pen is on positive Y axis
                tilt_y_rad = PI / 2.0;
            }
            if azimuth_angle == PI {
                // pen is on negative X axis
                tilt_x_rad = -PI / 2.0;
            }
            if azimuth_angle == 3.0 * PI / 2.0 {
                // pen is on negative Y axis
                tilt_y_rad = -PI / 2.0;
            }
            if azimuth_angle > 0.0 && azimuth_angle < PI / 2.0 {
                tilt_x_rad = PI / 2.0;
                tilt_y_rad = PI / 2.0;
            }
            if azimuth_angle > PI / 2.0 && azimuth_angle < PI {
                tilt_x_rad = -PI / 2.0;
                tilt_y_rad = PI / 2.0;
            }
            if azimuth_angle > PI && azimuth_angle < 3.0 * PI / 2.0 {
                tilt_x_rad = -PI / 2.0;
                tilt_y_rad = -PI / 2.0;
            }
            if azimuth_angle > 3.0 * PI / 2.0 && azimuth_angle < TAU {
                tilt_x_rad = PI / 2.0;
                tilt_y_rad = -PI / 2.0;
            }
        };

        if altitude_angle != 0.0 {
            let tan_alt = altitude_angle.tan();
            tilt_x_rad = f32::atan(f32::cos(azimuth_angle) / tan_alt);
            tilt_y_rad = f32::atan(f32::sin(azimuth_angle) / tan_alt);
        }
        PenInclinationTilt {
            tilt_x: f32::round(tilt_x_rad * rad_to_deg) as i32,
            tilt_y: f32::round(tilt_y_rad * rad_to_deg) as i32,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PenInfo {
    pub pressure: f32,            // 0.0..1.0
    pub tangential_pressure: f32, // -1.0..1.0
    pub inclination: PenInclination,
    pub twist: u16, // 0..359 degrees clockwise rotation
}

impl PenInfo {}

#[derive(Debug, Clone, PartialEq)]
pub struct TouchInfo {
    pub contact_geometry: Size,
    pub pressure: f32,
    // TODO: Phase?
}

#[derive(Debug, Clone, PartialEq)]
pub struct MouseInfo {
    pub wheel_delta: Vec2,
}

impl Default for PenInfo {
    fn default() -> Self {
        PenInfo {
            pressure: 0.5, // In the range zero to one, must be 0.5 when in active buttons state for hardware that doesn't support pressure, and 0 otherwise
            tangential_pressure: 0.0,
            twist: 0,
            inclination: PenInclination::from_angle(0.0, std::f32::consts::PI / 2.0),
        }
    }
}

impl Default for TouchInfo {
    fn default() -> Self {
        Self {
            pressure: 0.5, // In the range zero to one, must be 0.5 when in active buttons state for hardware that doesn't support pressure, and 0 otherwise
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
    Eraser,
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
    // This is a super-set of mouse events and stylus + touch events.
    pub pointer_id: u32,
    pub is_primary: bool,
    pub pointer_type: PointerType,

    // Maybe we should have microseconds here?  Should it be a u64 or a double?
    pub timestamp: u32, // Milliseconds of system uptime.  This just needs to be considered relative to other events.
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
            pointer_type: PointerType::Mouse(MouseInfo {
                wheel_delta: Vec2::ZERO,
            }),
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
