//! A number that moves to a new value over a short time instead of
//! jumping: a bar draining, a pane fading in, a ghost bar catching up.

/// How the motion speeds up and slows down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ease {
    /// The same speed all the way.
    Linear,
    /// Fast at first, settling gently (most motion in the interface).
    #[default]
    Out,
    /// Slow at both ends.
    InOut,
}

impl Ease {
    /// How far along the motion is at `t` (0 to 1) of its time.
    #[must_use]
    pub fn at(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::Out => 1.0 - (1.0 - t).powi(3),
            Self::InOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
        }
    }
}

/// A value easing towards a target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween {
    from: f32,
    to: f32,
    elapsed: f32,
    /// How long it waits before it sets off.
    delay: f32,
    duration: f32,
    ease: Ease,
}

impl Tween {
    /// A value at rest at `value`.
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            elapsed: 0.0,
            delay: 0.0,
            duration: 0.0,
            ease: Ease::Out,
        }
    }

    /// Where the value is now.
    #[must_use]
    pub fn value(&self) -> f32 {
        if self.is_settled() {
            return self.to;
        }
        if self.elapsed <= self.delay {
            return self.from;
        }
        let k = self.ease.at((self.elapsed - self.delay) / self.duration);
        self.from + (self.to - self.from) * k
    }

    /// Where the value is going.
    #[must_use]
    pub const fn target(&self) -> f32 {
        self.to
    }

    /// True when the value has arrived.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        self.elapsed >= self.delay + self.duration
    }

    /// Sets off from where the value is now towards `target`, arriving
    /// after `seconds`; a target it is already going to changes nothing.
    pub fn go_to(&mut self, target: f32, seconds: f32, ease: Ease) {
        self.go_to_after(target, 0.0, seconds, ease);
    }

    /// [`Tween::go_to`], holding still for `delay` seconds first.
    pub fn go_to_after(&mut self, target: f32, delay: f32, seconds: f32, ease: Ease) {
        if target == self.to {
            return;
        }
        self.from = self.value();
        self.to = target;
        self.elapsed = 0.0;
        self.delay = delay.max(0.0);
        self.duration = seconds.max(0.0);
        self.ease = ease;
    }

    /// Puts the value at `value` at once.
    pub fn jump_to(&mut self, value: f32) {
        *self = Self::new(value);
    }

    /// Moves on by `seconds`.
    pub fn tick(&mut self, seconds: f32) {
        if !self.is_settled() {
            self.elapsed = (self.elapsed + seconds.max(0.0)).min(self.delay + self.duration);
        }
    }
}

impl Default for Tween {
    fn default() -> Self {
        Self::new(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eases_start_at_zero_and_end_at_one() {
        for ease in [Ease::Linear, Ease::Out, Ease::InOut] {
            assert_eq!(ease.at(0.0), 0.0);
            assert_eq!(ease.at(1.0), 1.0);
            assert_eq!(ease.at(2.0), 1.0);
        }
        assert!(Ease::Out.at(0.5) > 0.5);
        assert_eq!(Ease::InOut.at(0.5), 0.5);
    }

    #[test]
    fn a_tween_arrives_after_its_time() {
        let mut t = Tween::new(1.0);
        t.go_to(0.0, 0.5, Ease::Linear);
        assert_eq!(t.value(), 1.0);
        t.tick(0.25);
        assert_eq!(t.value(), 0.5);
        t.tick(1.0);
        assert!(t.is_settled());
        assert_eq!(t.value(), 0.0);
    }

    #[test]
    fn a_new_target_sets_off_from_where_it_is() {
        let mut t = Tween::new(0.0);
        t.go_to(1.0, 1.0, Ease::Linear);
        t.tick(0.5);
        t.go_to(0.0, 1.0, Ease::Linear);
        assert_eq!(t.value(), 0.5);
        t.tick(0.5);
        assert_eq!(t.value(), 0.25);
        // The same target again does not restart it.
        t.go_to(0.0, 1.0, Ease::Linear);
        assert_eq!(t.value(), 0.25);
    }

    #[test]
    fn a_delayed_tween_holds_then_moves() {
        let mut t = Tween::new(1.0);
        t.go_to_after(0.0, 0.5, 0.5, Ease::Linear);
        t.tick(0.4);
        assert_eq!(t.value(), 1.0);
        t.tick(0.35);
        assert_eq!(t.value(), 0.5);
        t.tick(1.0);
        assert!(t.is_settled());
        assert_eq!(t.value(), 0.0);
    }

    #[test]
    fn no_time_is_a_jump() {
        let mut t = Tween::new(3.0);
        t.go_to(7.0, 0.0, Ease::Out);
        assert_eq!(t.value(), 7.0);
        t.jump_to(2.0);
        assert!(t.is_settled());
        assert_eq!(t.value(), 2.0);
    }
}
