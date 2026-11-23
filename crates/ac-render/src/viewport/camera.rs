//! Where the viewport scenes are seen from.
//!
//! Every viewport camera looks down +Y (north) from its eye, with +Z up and
//! no turn, through a 45 degree vertical field of view; only the character
//! selection's camera tilts, a little down. Only the eye moves: the figure
//! stands at the origin and the eye is placed in front of it (negative Y)
//! and at about chest or face height. Distances are metres.
//!
//! - The inventory figure's eye is fixed per heritage (larger bodies are
//!   seen from further away).
//! - The character-creation preview has a close eye (the face) and a far
//!   eye (the whole body) per heritage, and slides between them in 0.6 s.
//! - The examine portrait's eye is fitted to the creature's box.
//! - The character selection's eye stands further back than creation's
//!   far eye and higher, tilted down to the same point of the body, so the
//!   whole figure shows from about chest height (creation's far eye, level
//!   at the hip and close, looks up at a figure filling a tall frame).

use ac_core::chargen::ChargenHeritageGroup as H;
use glam::{Mat4, Vec3};

/// The vertical field of view of every viewport camera, radians (45
/// degrees).
pub const FIELD_OF_VIEW: f32 = std::f32::consts::FRAC_PI_4;
/// The near plane, metres.
pub const NEAR: f32 = 0.1;
/// The far plane of the figure and preview cameras, metres (their scenes
/// are small).
pub const FIGURE_FAR: f32 = 50.0;
/// The far plane of the examine camera, metres (a large creature's eye can
/// be far away).
pub const EXAMINE_FAR: f32 = 2048.0;

/// A viewport camera: an eye looking down +Y.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportCamera {
    /// Where the scene is seen from, metres.
    pub eye: Vec3,
    /// Vertical field of view, radians.
    pub fov: f32,
    /// Near plane, metres.
    pub near: f32,
    /// Far plane, metres.
    pub far: f32,
    /// Width over height of the viewport.
    pub aspect: f32,
    /// How far it looks down from level, radians.
    pub tilt: f32,
}

impl ViewportCamera {
    /// A camera at `eye` with the far plane `far`.
    #[must_use]
    pub fn new(eye: Vec3, far: f32) -> Self {
        Self {
            eye,
            fov: FIELD_OF_VIEW,
            near: NEAR,
            far,
            aspect: 1.0,
            tilt: 0.0,
        }
    }

    /// The camera looking `tilt` radians down from level.
    #[must_use]
    pub fn tilted(self, tilt: f32) -> Self {
        Self { tilt, ..self }
    }

    /// World to view: from the eye toward +Y tilted down by `tilt`, +Z up.
    #[must_use]
    pub fn view(&self) -> Mat4 {
        let ahead = Vec3::new(0.0, self.tilt.cos(), -self.tilt.sin());
        glam::camera::rh::view::look_at_mat4(self.eye, self.eye + ahead, Vec3::Z)
    }

    /// View to clip (depth 0..1); an aspect that is not positive counts as
    /// 1.
    #[must_use]
    pub fn projection(&self) -> Mat4 {
        let aspect = if self.aspect > 0.0 { self.aspect } else { 1.0 };
        glam::camera::rh::proj::directx::perspective(self.fov, aspect, self.near, self.far)
    }
}

/// How much further back the character selection's eye stands than
/// creation's far eye.
const STAGE_BACK: f32 = 1.3;
/// How much higher it stands, metres for each metre back from the figure.
const STAGE_RISE: f32 = 0.18;

/// The character selection's eye for a heritage and its tilt down
/// (radians): creation's far eye moved back and up, looking at the same
/// height of the body.
#[must_use]
pub fn stage_eye(heritage: u32) -> (Vec3, f32) {
    let far = chargen_zoomed_out_eye(heritage);
    let back = -far.y * STAGE_BACK;
    let rise = back * STAGE_RISE;
    (Vec3::new(far.x, -back, far.z + rise), (rise / back).atan())
}

/// The inventory figure's eye before the heritage is known.
pub const DOLL_DEFAULT_EYE: Vec3 = Vec3::new(0.12, -2.4, 0.88);

/// The inventory figure's eye for a heritage: further back for the larger
/// bodies.
#[must_use]
pub fn doll_eye(heritage: u32) -> Vec3 {
    match H(heritage) {
        H::GEARKNIGHT | H::TUMEROK => Vec3::new(0.12, -3.0, 0.88),
        H::LUGIAN => Vec3::new(0.12, -3.4, 1.0),
        H::EMPYREAN | H::OLTHOI | H::OLTHOI_ACID => Vec3::new(0.12, -3.4, 0.88),
        _ => DOLL_DEFAULT_EYE,
    }
}

/// The creation preview's close (zoomed-in) eye for a heritage.
#[must_use]
pub fn chargen_default_eye(heritage: u32) -> Vec3 {
    match H(heritage) {
        H::OLTHOI => Vec3::new(0.0, -1.85, 1.85),
        H::OLTHOI_ACID => Vec3::new(0.0, -3.05, 2.75),
        H::TUMEROK => Vec3::new(0.0, -0.85, 1.65),
        _ => Vec3::new(0.0, -0.55, 1.65),
    }
}

/// The creation preview's far (zoomed-out) eye for a heritage.
#[must_use]
pub fn chargen_zoomed_out_eye(heritage: u32) -> Vec3 {
    match H(heritage) {
        H::OLTHOI => Vec3::new(0.0, -3.8, 1.15),
        H::OLTHOI_ACID => Vec3::new(0.0, -5.7, 1.65),
        _ => Vec3::new(0.0, -2.5, 0.95),
    }
}

/// The barber preview's eye for a heritage: a little further down and
/// back for the bodies without a face to choose.
#[must_use]
pub fn barber_eye(heritage: u32) -> Vec3 {
    match H(heritage) {
        H::GEARKNIGHT | H::OLTHOI | H::OLTHOI_ACID => Vec3::new(0.0, -0.8, 1.5),
        _ => Vec3::new(0.0, -0.65, 1.7),
    }
}

/// Seconds the creation preview's turn buttons take for a full turn.
pub const ROTATION_SECONDS_PER_REVOLUTION: f32 = 3.0;

/// Seconds the creation preview's zoom slides for.
pub const ZOOM_TWEEN_SECONDS: f32 = 0.6;

/// How far back the examine eye stands per metre of the box it frames
/// (`1 / (2 tan(22.5 degrees))`: the distance at which a 45 degree view is
/// one metre tall).
pub const FIT_DISTANCE_FACTOR: f32 = 1.207_106_8;

/// The examine portrait's eye for a box (`min`, `max`, the figure's own
/// axes after its turn toward the viewer) in a viewport of `aspect`: in
/// front of the box's centre, far enough back that its height, or its
/// width at this aspect, fills the view, plus half its depth.
#[must_use]
pub fn examine_eye(min: Vec3, max: Vec3, aspect: f32) -> Vec3 {
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        1.0
    };
    let (low, high) = (min.min(max), min.max(max));
    let span = (high - low).max(Vec3::splat(0.001));
    let center = (low + high) * 0.5;
    let fitted_vertical = span.z.max(span.x / aspect);
    let distance = fitted_vertical * FIT_DISTANCE_FACTOR + span.y * 0.5;
    Vec3::new(center.x, center.y - distance, center.z)
}

/// The box the examine eye frames before a creature is fitted.
pub const EXAMINE_DEFAULT_BOX: (Vec3, Vec3) =
    (Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.5, 0.5, 2.0));

#[cfg(test)]
mod tests {
    //! Cases from OpenAC `tests/AcDream.App.Tests/Rendering/DollCameraTests.cs`,
    //! `ChargenPreviewCameraTests.cs`, `PaperdollHeritagePresentationTests.cs`
    //! and `CreatureAppraisalPresentationTests.cs` (the camera rows).
    use super::*;

    #[test]
    fn the_selection_eye_looks_down_at_the_height_the_far_eye_sees() {
        for heritage in [0, 1, 7, 12] {
            let far = chargen_zoomed_out_eye(heritage);
            let (eye, tilt) = stage_eye(heritage);
            assert!(eye.y < far.y && eye.z > far.z, "{heritage}: {eye}");
            assert!(tilt > 0.0 && tilt < 0.25, "{heritage}: {tilt}");
            // Down the tilt, the camera meets the figure's plane (y = 0) at
            // the far eye's height.
            let at = eye.z + eye.y * tilt.tan();
            assert!((at - far.z).abs() < 1e-4, "{heritage}: {at}");
        }
        assert_eq!(ViewportCamera::new(Vec3::ZERO, 1.0).tilt, 0.0);
    }

    fn close(a: f32, b: f32, places: i32) -> bool {
        (a - b).abs() < 0.5 * 10f32.powi(-places)
    }

    fn eye_of(view: Mat4) -> Vec3 {
        view.inverse().w_axis.truncate()
    }

    #[test]
    fn the_doll_eye_is_retail_verbatim_and_round_trips_the_view() {
        let mut camera = ViewportCamera::new(DOLL_DEFAULT_EYE, FIGURE_FAR);
        camera.aspect = 100.0 / 214.0;
        let eye = eye_of(camera.view());
        assert!(close(eye.x, 0.12, 3) && close(eye.y, -2.4, 3) && close(eye.z, 0.88, 3));
    }

    #[test]
    fn the_look_axis_is_pure_plus_y() {
        for eye in [
            DOLL_DEFAULT_EYE,
            chargen_default_eye(12),
            Vec3::new(3.0, -9.0, 1.0),
        ] {
            let view = ViewportCamera::new(eye, FIGURE_FAR).view();
            let forward = view.inverse().transform_vector3(Vec3::NEG_Z);
            assert!(forward.abs_diff_eq(Vec3::Y, 1e-4), "{forward:?}");
        }
    }

    #[test]
    fn the_projection_is_finite_and_uses_the_aspect() {
        let mut camera = ViewportCamera::new(DOLL_DEFAULT_EYE, FIGURE_FAR);
        camera.aspect = 1.5;
        let p = camera.projection();
        assert!(p.x_axis.x.is_finite());
        assert_ne!(p.z_axis.w, 0.0);
        let mut square = camera;
        square.aspect = 1.0;
        assert!(close(p.x_axis.x * 1.5, square.projection().x_axis.x, 5));
        let mut broken = camera;
        broken.aspect = 0.0;
        assert_eq!(broken.projection(), square.projection());
    }

    #[test]
    fn the_doll_eye_follows_the_heritage() {
        // PaperdollHeritagePresentationTests: larger bodies stand further
        // back; the humans keep the default eye.
        for h in [1, 2, 3, 4, 5, 10, 11, 0, 99] {
            assert_eq!(doll_eye(h), DOLL_DEFAULT_EYE, "heritage {h}");
        }
        assert_eq!(doll_eye(6), Vec3::new(0.12, -3.0, 0.88));
        assert_eq!(doll_eye(7), Vec3::new(0.12, -3.0, 0.88));
        assert_eq!(doll_eye(8), Vec3::new(0.12, -3.4, 1.0));
        for h in [9, 12, 13] {
            assert_eq!(doll_eye(h), Vec3::new(0.12, -3.4, 0.88), "heritage {h}");
        }
    }

    #[test]
    fn the_chargen_eyes_match_the_retail_literals() {
        let rows = [
            (1, Vec3::new(0.0, -0.55, 1.65)),
            (2, Vec3::new(0.0, -0.55, 1.65)),
            (6, Vec3::new(0.0, -0.55, 1.65)),
            (11, Vec3::new(0.0, -0.55, 1.65)),
            (7, Vec3::new(0.0, -0.85, 1.65)),
            (12, Vec3::new(0.0, -1.85, 1.85)),
            (13, Vec3::new(0.0, -3.05, 2.75)),
        ];
        for (h, eye) in rows {
            assert!(
                chargen_default_eye(h).abs_diff_eq(eye, 1e-4),
                "heritage {h}"
            );
        }
        let far = [
            (1, Vec3::new(0.0, -2.5, 0.95)),
            (7, Vec3::new(0.0, -2.5, 0.95)),
            (12, Vec3::new(0.0, -3.8, 1.15)),
            (13, Vec3::new(0.0, -5.7, 1.65)),
        ];
        for (h, eye) in far {
            assert!(
                chargen_zoomed_out_eye(h).abs_diff_eq(eye, 1e-4),
                "heritage {h}"
            );
        }
        assert_eq!(chargen_default_eye(0), chargen_default_eye(1));
        assert_eq!(ROTATION_SECONDS_PER_REVOLUTION, 3.0);
        assert_eq!(ZOOM_TWEEN_SECONDS, 0.6);
    }

    #[test]
    fn the_examine_eye_fits_the_vertical_span() {
        // CameraFitsRetailVerticalSpanAndKeepsIdentityLookDirection.
        let eye = examine_eye(Vec3::new(-1.0, -0.5, 0.0), Vec3::new(1.0, 0.5, 4.0), 2.0);
        assert!(close(eye.x, 0.0, 5));
        assert!(close(eye.y, -(4.0 * FIT_DISTANCE_FACTOR + 0.5), 5));
        assert!(close(eye.z, 2.0, 5));
    }

    #[test]
    fn the_examine_eye_uses_the_aspect_for_wide_creatures() {
        // CameraUsesViewportAspectWhenCreatureIsWiderThanTall.
        let eye = examine_eye(Vec3::new(-4.0, -0.5, 0.0), Vec3::new(4.0, 0.5, 2.0), 1.0);
        assert!(close(eye.y, -(8.0 * FIT_DISTANCE_FACTOR + 0.5), 5));
        // The factor is the distance at which the 45 degree view is a metre
        // tall (OpenAC writes it 1.20710683f).
        assert!((FIT_DISTANCE_FACTOR - 0.5 / (FIELD_OF_VIEW / 2.0).tan()).abs() < 1e-6);
        // A bad aspect counts as square; a flat box keeps a minimum span.
        assert_eq!(
            examine_eye(Vec3::ZERO, Vec3::ONE, f32::NAN),
            examine_eye(Vec3::ZERO, Vec3::ONE, 1.0)
        );
        assert!(examine_eye(Vec3::ZERO, Vec3::ZERO, 1.0).y < 0.0);
    }
}
