//! The OS cursor for the window: the retail cursor art the UI decided on
//! ([`ac_ui::cursor`]), decoded from the dats with its key colour made
//! transparent and handed to the window as a custom cursor; a global
//! cursor the dats lack falls back to the nearest system cursor. With the
//! modern interface the global cursors are its own
//! ([`ac_ui::modern::cursors`]).

use std::collections::{HashMap, HashSet};

use ac_dat::DatCollection;
use ac_dat::objects::{Palette, RenderSurface};
use ac_ui::cursor::{
    CursorFeedback, CursorLayer, GlobalCursorKind, knock_out_key_color, plan_application,
    resolve_global_cursor,
};
use ac_ui::element_info::CursorMedia;
use winit::event_loop::ActiveEventLoop;
use winit::window::{CursorIcon, CustomCursor, Window};

/// What is on the window now.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Shown {
    Custom(CursorMedia),
    Modern(GlobalCursorKind),
    System(CursorIcon),
}

/// Applies the UI's cursor decision to the window (OpenAC
/// `RetailCursorManager`).
#[derive(Debug, Default)]
pub struct CursorManager {
    previous: Option<(GlobalCursorKind, Option<CursorMedia>)>,
    globals: HashMap<GlobalCursorKind, Option<CursorMedia>>,
    customs: HashMap<CursorMedia, Option<CustomCursor>>,
    reported: HashSet<GlobalCursorKind>,
    shown: Option<Shown>,
    /// The modern interface's own global cursors are shown.
    modern: bool,
    moderns: HashMap<GlobalCursorKind, Option<CustomCursor>>,
}

impl CursorManager {
    /// No cursor applied yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The modern interface's global cursors (`true`) or retail's from now
    /// on.
    pub fn set_modern(&mut self, on: bool) {
        if self.modern != on {
            self.modern = on;
            // Everything is shown again in the new look.
            self.previous = None;
            self.shown = None;
        }
    }

    /// Show `feedback` on `window`, changing only the layers that changed.
    /// Returns a line for the log the first time a global cursor falls
    /// back to the system one.
    pub fn apply(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: &Window,
        dats: &DatCollection,
        feedback: CursorFeedback,
    ) -> Vec<String> {
        let mut notices = Vec::new();
        for layer in plan_application(self.previous, feedback.global, feedback.cursor) {
            let custom = match (layer, feedback.cursor) {
                (CursorLayer::Widget, Some(c)) => self.custom(event_loop, dats, c).map(|k| (c, k)),
                _ => None,
            };
            match custom {
                Some((media, cursor)) => self.show_custom(window, media, cursor),
                None => self.show_global(event_loop, window, dats, feedback.global, &mut notices),
            }
        }
        self.previous = Some((feedback.global, feedback.cursor));
        notices
    }

    fn show_global(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: &Window,
        dats: &DatCollection,
        kind: GlobalCursorKind,
        notices: &mut Vec<String>,
    ) {
        if self.modern
            && let Some(cursor) = self
                .moderns
                .entry(kind)
                .or_insert_with(|| modern_cursor(event_loop, kind))
                .clone()
        {
            if self.shown != Some(Shown::Modern(kind)) {
                window.set_cursor(cursor);
                self.shown = Some(Shown::Modern(kind));
            }
            return;
        }
        let media = *self
            .globals
            .entry(kind)
            .or_insert_with(|| resolve_global_cursor(dats, kind));
        if let Some(media) = media
            && let Some(cursor) = self.custom(event_loop, dats, media)
        {
            self.show_custom(window, media, cursor);
            return;
        }
        if self.reported.insert(kind) {
            notices.push(format!(
                "cursor: the retail {kind:?} cursor is not in the dats; using the system cursor"
            ));
        }
        let icon = system_cursor(kind);
        if self.shown != Some(Shown::System(icon)) {
            window.set_cursor(icon);
            self.shown = Some(Shown::System(icon));
        }
    }

    fn show_custom(&mut self, window: &Window, media: CursorMedia, cursor: CustomCursor) {
        if self.shown == Some(Shown::Custom(media)) {
            return;
        }
        window.set_cursor(cursor);
        self.shown = Some(Shown::Custom(media));
    }

    /// The window cursor for `media`, made once; `None` when the surface
    /// is missing or the platform refuses it.
    fn custom(
        &mut self,
        event_loop: &ActiveEventLoop,
        dats: &DatCollection,
        media: CursorMedia,
    ) -> Option<CustomCursor> {
        self.customs
            .entry(media)
            .or_insert_with(|| {
                let (rgba, w, h) = cursor_image(dats, media.file)?;
                let source = CustomCursor::from_rgba(
                    rgba,
                    u16::try_from(w).ok()?,
                    u16::try_from(h).ok()?,
                    u16::try_from(media.hotspot_x.clamp(0, w as i32 - 1)).ok()?,
                    u16::try_from(media.hotspot_y.clamp(0, h as i32 - 1)).ok()?,
                )
                .ok()?;
                Some(event_loop.create_custom_cursor(source))
            })
            .clone()
    }
}

/// The modern interface's cursor for `kind`; `None` when the platform
/// refuses it.
fn modern_cursor(event_loop: &ActiveEventLoop, kind: GlobalCursorKind) -> Option<CustomCursor> {
    use ac_ui::modern::cursors::{SIZE, draw};
    let c = draw(kind, SIZE);
    let side = u16::try_from(c.size).ok()?;
    let source = CustomCursor::from_rgba(
        c.rgba,
        side,
        side,
        u16::try_from(c.hot_spot.0).ok()?,
        u16::try_from(c.hot_spot.1).ok()?,
    )
    .ok()?;
    Some(event_loop.create_custom_cursor(source))
}

/// Cursor art `file` as RGBA with its key colour knocked out.
fn cursor_image(dats: &DatCollection, file: u32) -> Option<(Vec<u8>, u32, u32)> {
    let rs = dats.get::<RenderSurface>(file).ok()??;
    let palette = if rs.default_palette_id == 0 {
        None
    } else {
        dats.get::<Palette>(rs.default_palette_id).ok().flatten()
    };
    let mut decoded = ac_dat::texture::decode(
        &rs,
        palette.as_ref(),
        ac_dat::texture::DecodeOptions::default(),
    )
    .ok()?;
    if decoded.width == 0 || decoded.height == 0 || decoded.rgba.is_empty() {
        return None;
    }
    knock_out_key_color(&mut decoded.rgba);
    Some((decoded.rgba, decoded.width, decoded.height))
}

/// The system cursor standing in for a global cursor the dats lack.
fn system_cursor(kind: GlobalCursorKind) -> CursorIcon {
    use GlobalCursorKind as G;
    match kind {
        G::Use | G::UseFound => CursorIcon::Pointer,
        G::TargetPending => CursorIcon::Crosshair,
        G::TargetValid => CursorIcon::Move,
        G::TargetInvalid => CursorIcon::NotAllowed,
        _ => CursorIcon::Default,
    }
}
