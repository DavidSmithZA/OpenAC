//! The modern Options window's rows and values.
//!
//! Each row of [`spec::GROUPS`] takes what it is from the game's option
//! pages: a checkbox is a switch, a slider keeps its range and step, a menu
//! keeps its choices; the default is theirs too. Settings the game
//! marks as OpenAC's own carry the OPENAC tag.
//!
//! The values are a copy of the live ones, as the game's window keeps:
//! a change is written to the copy and made live at once; the copy taken
//! when the window opened (or last applied) is what Revert goes back to,
//! and closing without Apply reverts.

use ac_ui::panels::options::OptionValue;
use ac_ui::panels::options::keys::{
    CameraField, LiveValues, MixerField, OptionKey, RenderPackField, Section as Stored,
};
use ac_ui::panels::options::spec::{self as retail, ConfigRow};

use super::spec::{self, Action, Group, Item, Section, Words};

/// How a slider shows its value.
#[derive(Debug, Clone, PartialEq)]
pub enum Shown {
    /// The value as a share of 1, in percent (volumes).
    Percent,
    /// Where it sits between its ends, in percent, with the ends' words.
    Between(String, String),
    /// The number, with this many decimals and these words after.
    Number(usize, &'static str),
}

/// What a row is.
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    /// A switch.
    Switch {
        /// It shows the stored value's opposite.
        invert: bool,
    },
    /// A slider in the setting's own unit.
    Slider {
        /// Low end.
        min: f64,
        /// High end.
        max: f64,
        /// Step (0: none).
        step: f64,
        /// How it shows its value.
        shown: Shown,
    },
    /// One of a few choices.
    Choice {
        /// Each choice's value.
        values: Vec<OptionValue>,
        /// Each choice's words.
        labels: Vec<String>,
    },
    /// A switch and a volume (0 to 1) on one line.
    Volume {
        /// The switch's key.
        switch: OptionKey,
    },
    /// A button.
    Button(Action, &'static str),
    /// The chat tabs against the message types.
    ChatGrid,
}

/// One row of the window.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Its section.
    pub section: Section,
    /// Its group (an index of [`spec::GROUPS`]).
    pub group: usize,
    /// What it edits (none for a button or the grid).
    pub key: Option<OptionKey>,
    /// Its name.
    pub label: String,
    /// What it does.
    pub desc: String,
    /// Not in the original client.
    pub openac: bool,
    /// What it is.
    pub control: Control,
    /// Its default (and its switch's, for a volume).
    pub default: Option<OptionValue>,
    /// The volume's switch's default.
    pub switch_default: Option<OptionValue>,
    /// Greyed while the original client's mixer is on.
    pub mixer_dimmed: bool,
}

impl Row {
    /// The keys it edits.
    #[must_use]
    pub fn keys(&self) -> Vec<OptionKey> {
        let mut keys: Vec<OptionKey> = self.key.into_iter().collect();
        if let Control::Volume { switch } = self.control {
            keys.push(switch);
        }
        keys
    }

    /// It is one of the atmospheric rendering's own settings.
    #[must_use]
    pub fn is_atmospheric_setting(&self) -> bool {
        matches!(
            self.key,
            Some(OptionKey::RenderPack(RenderPackField::Setting(_)))
        )
    }
}

/// Where the rows take their captions and choices from.
pub struct Sources<'a> {
    /// A string of the game's by key (`None` when the dats lack it).
    pub strings: &'a dyn Fn(&str) -> Option<String>,
    /// The resolutions the display offers.
    pub resolutions: &'a [String],
}

impl std::fmt::Debug for Sources<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sources")
            .field("resolutions", &self.resolutions)
            .finish_non_exhaustive()
    }
}

fn retail_row(rows: &[ConfigRow], key: OptionKey) -> Option<ConfigRow> {
    rows.iter().copied().find(|r| match *r {
        ConfigRow::Toggle { target, .. }
        | ConfigRow::Slider { target, .. }
        | ConfigRow::Menu { target, .. }
        | ConfigRow::ExplicitToggle { target, .. }
        | ConfigRow::ExplicitSlider { target, .. }
        | ConfigRow::ExplicitMenu { target, .. } => target == key,
        ConfigRow::Trio {
            toggle_target,
            slider_target,
            ..
        } => toggle_target == key || slider_target == key,
        ConfigRow::Resolution { .. } => {
            key == OptionKey::Display(ac_ui::panels::options::keys::DisplayField::Resolution)
        }
        ConfigRow::Header(_) | ConfigRow::Separator | ConfigRow::ExplicitHeader(_) => false,
    })
}

fn decimals(step: f64) -> usize {
    if step <= 0.0 || step >= 1.0 {
        0
    } else if step >= 0.1 {
        1
    } else {
        2
    }
}

/// What a game row is, its default and whether OpenAC added it.
fn control_of(
    row: ConfigRow,
    key: OptionKey,
    words: Words,
    src: &Sources<'_>,
) -> Option<(Control, Option<OptionValue>, bool, bool)> {
    let say = |k: &str| (src.strings)(k).unwrap_or_else(|| k.to_owned());
    Some(match row {
        ConfigRow::Toggle { default, .. } => (
            Control::Switch {
                invert: words.invert,
            },
            Some(OptionValue::Bool(default)),
            false,
            false,
        ),
        ConfigRow::ExplicitToggle {
            default,
            dimmed_by_retail_mixer,
            ..
        } => (
            Control::Switch {
                invert: words.invert,
            },
            Some(OptionValue::Bool(default)),
            true,
            dimmed_by_retail_mixer,
        ),
        ConfigRow::Slider {
            min,
            max,
            default,
            range,
            ..
        } => {
            let shown = match (key, range) {
                (
                    OptionKey::Display(ac_ui::panels::options::keys::DisplayField::FieldOfView),
                    _,
                ) => Shown::Number(0, ""),
                (_, Some((low, high))) => Shown::Between(say(low), say(high)),
                _ => Shown::Between("Slow".to_owned(), "Fast".to_owned()),
            };
            (
                Control::Slider {
                    min: f64::from(min),
                    max: f64::from(max),
                    step: 0.0,
                    shown,
                },
                Some(OptionValue::Float(default)),
                false,
                false,
            )
        }
        ConfigRow::ExplicitSlider {
            min,
            max,
            step,
            default,
            value_suffix,
            dimmed_by_retail_mixer,
            ..
        } => (
            Control::Slider {
                min,
                max,
                step,
                shown: Shown::Number(decimals(step), value_suffix.unwrap_or("")),
            },
            Some(OptionValue::Float(default as f32)),
            true,
            dimmed_by_retail_mixer,
        ),
        ConfigRow::Menu {
            choices,
            payloads,
            default,
            ..
        } => {
            let values = (0..choices.len())
                .map(|i| {
                    OptionValue::Int(
                        payloads
                            .and_then(|p| p.get(i).copied())
                            .unwrap_or(i32::try_from(i).unwrap_or(0)),
                    )
                })
                .collect();
            let labels = choices.iter().map(|c| say(c)).collect();
            (
                Control::Choice { values, labels },
                Some(OptionValue::Int(default)),
                false,
                false,
            )
        }
        ConfigRow::ExplicitMenu {
            choices, default, ..
        } => {
            let shown: Vec<_> = choices.iter().filter(|c| c.enabled).collect();
            (
                Control::Choice {
                    values: shown
                        .iter()
                        .map(|c| OptionValue::Text(c.id.to_owned()))
                        .collect(),
                    labels: shown.iter().map(|c| c.label.to_owned()).collect(),
                },
                Some(OptionValue::Text(default.to_owned())),
                true,
                false,
            )
        }
        ConfigRow::Resolution { .. } => (
            Control::Choice {
                values: src
                    .resolutions
                    .iter()
                    .map(|r| OptionValue::Text(r.clone()))
                    .collect(),
                labels: src
                    .resolutions
                    .iter()
                    .map(|r| r.replace('x', " x "))
                    .collect(),
            },
            None,
            false,
            false,
        ),
        ConfigRow::Trio { .. }
        | ConfigRow::Header(_)
        | ConfigRow::Separator
        | ConfigRow::ExplicitHeader(_) => return None,
    })
}

/// The caption of an OpenAC row the window has no words of its own for.
fn retail_label(row: ConfigRow) -> Option<&'static str> {
    match row {
        ConfigRow::ExplicitToggle { label, .. }
        | ConfigRow::ExplicitSlider { label, .. }
        | ConfigRow::ExplicitMenu { label, .. } => Some(label),
        _ => None,
    }
}

/// Every row, in the window's order.
#[must_use]
pub fn rows(src: &Sources<'_>) -> Vec<Row> {
    let config = retail::config_rows();
    let mut out = Vec::new();
    for (g, group) in spec::GROUPS.iter().enumerate() {
        for item in group.items {
            push_item(&mut out, g, group, *item, &config, src);
        }
    }
    out
}

fn push_item(
    out: &mut Vec<Row>,
    g: usize,
    group: &Group,
    item: Item,
    config: &[ConfigRow],
    src: &Sources<'_>,
) {
    let base = |key, words: Words, control, default| Row {
        section: group.section,
        group: g,
        key,
        label: words.label.to_owned(),
        desc: words.desc.to_owned(),
        openac: false,
        control,
        default,
        switch_default: None,
        mixer_dimmed: false,
    };
    match item {
        Item::Key(OptionKey::Character(id), words) => {
            let default =
                ac_runtime::character::option(id).map(|e| OptionValue::Bool(e.client_default));
            out.push(base(
                Some(OptionKey::Character(id)),
                words,
                Control::Switch {
                    invert: words.invert,
                },
                default,
            ));
        }
        Item::Key(key, words) => {
            let Some(row) = retail_row(config, key) else {
                return;
            };
            let Some((control, default, openac, dimmed)) = control_of(row, key, words, src) else {
                return;
            };
            let mut r = base(Some(key), words, control, default);
            r.openac = openac || is_openac(key);
            r.mixer_dimmed = dimmed;
            out.push(r);
        }
        Item::Volume(switch, volume, words) => {
            let (switch_default, volume_default) = match retail_row(config, volume) {
                Some(ConfigRow::Trio {
                    toggle_default,
                    slider_default,
                    ..
                }) => (toggle_default, slider_default),
                _ => (true, 1.0),
            };
            let mut r = base(
                Some(volume),
                words,
                Control::Volume { switch },
                Some(OptionValue::Float(volume_default)),
            );
            r.switch_default = Some(OptionValue::Bool(switch_default));
            out.push(r);
        }
        Item::Atmospheric => {
            for row in config {
                let (ConfigRow::ExplicitToggle { target, .. }
                | ConfigRow::ExplicitSlider { target, .. }
                | ConfigRow::ExplicitMenu { target, .. }) = *row
                else {
                    continue;
                };
                if !matches!(target, OptionKey::RenderPack(RenderPackField::Setting(_))) {
                    continue;
                }
                let label = retail_label(*row).unwrap_or("");
                let words = Words {
                    label,
                    desc: "",
                    invert: false,
                };
                if let Some((control, default, _, _)) = control_of(*row, target, words, src) {
                    let mut r = base(Some(target), words, control, default);
                    r.openac = true;
                    out.push(r);
                }
            }
        }
        Item::ChatGrid => out.push(base(
            None,
            Words {
                label: "What each tab shows",
                desc: "",
                invert: false,
            },
            Control::ChatGrid,
            None,
        )),
        Item::Button(action, words, verb) => {
            let mut r = base(None, words, Control::Button(action, verb), None);
            r.openac = action == Action::EditBars;
            out.push(r);
        }
    }
}

/// Settings OpenAC added that the game's pages show as its own rows but
/// with the game's captions (the mouse turning pair).
fn is_openac(key: OptionKey) -> bool {
    matches!(
        key,
        OptionKey::Camera(
            CameraField::ModernMouseTurning | CameraField::BothMouseButtonsRunForward
        ) | OptionKey::Mixer(_)
            | OptionKey::RenderPack(_)
    )
}

/// The rows and their values.
#[derive(Debug, Clone, Default)]
pub struct OptionsModel {
    /// Every row.
    pub rows: Vec<Row>,
    /// The values now (live).
    pub live: LiveValues,
    /// The values Revert goes back to.
    pub saved: LiveValues,
}

impl OptionsModel {
    /// The rows over values `live`, nothing changed.
    #[must_use]
    pub fn new(rows: Vec<Row>, live: LiveValues) -> Self {
        Self {
            rows,
            saved: live.clone(),
            live,
        }
    }

    /// The value of `key` now.
    #[must_use]
    pub fn value(&self, key: OptionKey) -> Option<OptionValue> {
        self.live.read(key)
    }

    /// Write `value` to `key`; the stored section it changed.
    pub fn set(&mut self, key: OptionKey, value: &OptionValue) -> Option<Stored> {
        if self.live.read(key).as_ref() == Some(value) {
            return None;
        }
        self.live.write(key, value)
    }

    fn keys(&self) -> Vec<OptionKey> {
        let mut keys: Vec<OptionKey> = Vec::new();
        for r in &self.rows {
            for k in r.keys() {
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
        }
        keys
    }

    /// The keys whose value differs from the saved one.
    #[must_use]
    pub fn changed(&self) -> Vec<OptionKey> {
        self.keys()
            .into_iter()
            .filter(|k| self.live.read(*k) != self.saved.read(*k))
            .collect()
    }

    /// Row `row` differs from its saved value.
    #[must_use]
    pub fn row_changed(&self, row: &Row) -> bool {
        row.keys()
            .into_iter()
            .any(|k| self.live.read(k) != self.saved.read(k))
    }

    /// How many rows of `section` differ from their saved values.
    #[must_use]
    pub fn changed_in(&self, section: Section) -> usize {
        self.rows
            .iter()
            .filter(|r| r.section == section && self.row_changed(r))
            .count()
    }

    /// Every changed value back to the saved one: the stored sections
    /// changed, in order, without repeats.
    pub fn revert(&mut self) -> Vec<Stored> {
        let mut touched = Vec::new();
        for key in self.changed() {
            let Some(value) = self.saved.read(key) else {
                continue;
            };
            if let Some(s) = self.live.write(key, &value)
                && !touched.contains(&s)
            {
                touched.push(s);
            }
        }
        touched
    }

    /// Every row of `section` (every row: `None`) to its default: the
    /// stored sections changed. The atmospheric quality goes first, so
    /// its own settings land on the preset it picks.
    pub fn defaults(&mut self, section: Option<Section>) -> Vec<Stored> {
        let all: Vec<usize> = (0..self.rows.len())
            .filter(|&i| section.is_none_or(|s| s == self.rows[i].section))
            .collect();
        self.defaults_of(&all)
    }

    /// The rows `rows` (indices) back to their defaults, as
    /// [`Self::defaults`].
    pub fn defaults_of(&mut self, rows: &[usize]) -> Vec<Stored> {
        let mut pairs: Vec<(OptionKey, OptionValue)> = Vec::new();
        for r in rows.iter().filter_map(|&i| self.rows.get(i)) {
            if let (Some(k), Some(v)) = (r.key, &r.default) {
                pairs.push((k, v.clone()));
            }
            if let (Control::Volume { switch }, Some(v)) = (&r.control, &r.switch_default) {
                pairs.push((*switch, v.clone()));
            }
        }
        // The atmospheric settings take their preset's values with the
        // quality, not their own defaults.
        pairs.retain(|(k, _)| !matches!(k, OptionKey::RenderPack(RenderPackField::Setting(_))));
        let mut touched = Vec::new();
        for (key, value) in pairs {
            if self.live.read(key).as_ref() == Some(&value) {
                continue;
            }
            if let Some(s) = self.live.write(key, &value)
                && !touched.contains(&s)
            {
                touched.push(s);
            }
        }
        touched
    }

    /// The values now are the saved ones (Apply).
    pub fn applied(&mut self) {
        self.saved = self.live.clone();
    }

    /// The row is greyed: the game makes it unavailable now, or the
    /// original mixer overrides it.
    #[must_use]
    pub fn unavailable(&self, row: &Row) -> bool {
        let Some(key) = row.key else {
            return false;
        };
        retail::unavailable(key, &self.live.settings)
            || (row.mixer_dimmed && self.live.settings.mixer.retail_mixer)
            || (matches!(key, OptionKey::Mixer(f) if f != MixerField::RetailMixer)
                && self.live.settings.mixer.retail_mixer)
    }

    /// The row shows: the atmospheric rendering's settings only while it
    /// is on.
    #[must_use]
    pub fn shows(&self, row: &Row) -> bool {
        if !row.is_atmospheric_setting() {
            return true;
        }
        self.live
            .read(OptionKey::RenderPack(RenderPackField::Quality))
            .is_some_and(|v| {
                v != OptionValue::Text(ac_host::settings::render_pack::RETAIL_PRESET_ID.to_owned())
            })
    }

    /// The rows (by index) whose words hold every word of `query`, in
    /// order.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<usize> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            return Vec::new();
        }
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, r)| self.shows(r))
            .filter(|(_, r)| {
                let hay = format!(
                    "{} {} {} {}",
                    r.label,
                    r.desc,
                    spec::GROUPS[r.group].title,
                    r.section.words()
                )
                .to_lowercase();
                words.iter().all(|w| hay.contains(w.as_str()))
            })
            .map(|(i, _)| i)
            .collect()
    }
}

/// A slider's value as words.
#[must_use]
pub fn slider_words(min: f64, max: f64, shown: &Shown, value: f64) -> String {
    match shown {
        Shown::Percent => format!("{}%", (value * 100.0).round() as i64),
        Shown::Between(..) => {
            let share = if max > min {
                (value - min) / (max - min)
            } else {
                0.0
            };
            format!("{}%", (share * 100.0).round() as i64)
        }
        Shown::Number(d, suffix) => format!("{value:.d$}{suffix}", d = *d),
    }
}

/// A value as a number for a slider.
#[must_use]
pub fn number(value: Option<&OptionValue>) -> f64 {
    match value {
        Some(OptionValue::Float(f)) => f64::from(*f),
        Some(OptionValue::Int(i)) => f64::from(*i),
        Some(OptionValue::Bool(b)) => f64::from(u8::from(*b)),
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_ui::panels::options::spec::CHARACTER_GROUPS;

    fn model() -> OptionsModel {
        let strings = |_: &str| None;
        let res = vec!["1920x1080".to_owned()];
        let src = Sources {
            strings: &strings,
            resolutions: &res,
        };
        OptionsModel::new(
            rows(&src),
            LiveValues {
                settings: ac_host::settings::PlayerSettings::default(),
                options1: ac_runtime::character::DEFAULT_OPTIONS1,
                options2: ac_runtime::character::DEFAULT_OPTIONS2,
            },
        )
    }

    #[test]
    fn every_setting_of_the_game_pages_is_listed_or_left_out_with_a_reason() {
        let m = model();
        let listed: Vec<OptionKey> = m.rows.iter().flat_map(Row::keys).collect();
        let left = |k: OptionKey| spec::LEFT_OUT.iter().any(|(l, _)| *l == k);
        for g in CHARACTER_GROUPS {
            for r in g.rows {
                let k = OptionKey::Character(r.id);
                assert!(
                    listed.contains(&k) || left(k),
                    "{} is missing",
                    r.retail_name
                );
            }
        }
        for row in retail::config_rows() {
            let keys: Vec<OptionKey> = match row {
                ConfigRow::Toggle { target, .. }
                | ConfigRow::Slider { target, .. }
                | ConfigRow::Menu { target, .. }
                | ConfigRow::ExplicitToggle { target, .. }
                | ConfigRow::ExplicitSlider { target, .. }
                | ConfigRow::ExplicitMenu { target, .. } => vec![target],
                ConfigRow::Trio {
                    toggle_target,
                    slider_target,
                    ..
                } => vec![toggle_target, slider_target],
                ConfigRow::Resolution { .. } => vec![OptionKey::Display(
                    ac_ui::panels::options::keys::DisplayField::Resolution,
                )],
                _ => vec![],
            };
            for k in keys {
                assert!(
                    listed.contains(&k) || left(k) || spec::is_retail_chat_filter(k),
                    "{k:?} is missing"
                );
            }
        }
    }

    #[test]
    fn a_change_shows_as_changed_until_reverted() {
        let mut m = model();
        let fog = OptionKey::Character(ac_runtime::character::option_id::DISABLE_DISTANCE_FOG);
        let was = m.value(fog);
        assert_eq!(
            m.set(fog, &OptionValue::Bool(true)),
            Some(Stored::Character(
                ac_runtime::character::option_id::DISABLE_DISTANCE_FOG
            ))
        );
        assert_eq!(m.changed(), vec![fog]);
        assert_eq!(m.changed_in(Section::Interface), 1);
        let back = m.revert();
        assert_eq!(back.len(), 1);
        assert_eq!(m.value(fog), was);
        assert!(m.changed().is_empty());
    }

    #[test]
    fn apply_keeps_a_change() {
        let mut m = model();
        let vsync = OptionKey::Display(ac_ui::panels::options::keys::DisplayField::VSync);
        let on = m.value(vsync) == Some(OptionValue::Bool(true));
        m.set(vsync, &OptionValue::Bool(!on));
        m.applied();
        assert!(m.changed().is_empty());
        assert!(m.revert().is_empty());
    }

    #[test]
    fn defaults_put_a_section_back() {
        let mut m = model();
        let scale = OptionKey::Display(ac_ui::panels::options::keys::DisplayField::UiScale);
        m.set(scale, &OptionValue::Float(150.0));
        let touched = m.defaults(Some(Section::Interface));
        assert!(touched.contains(&Stored::Display));
        assert_eq!(m.value(scale), Some(OptionValue::Float(100.0)));
    }

    #[test]
    fn search_finds_rows_by_any_of_their_words() {
        let m = model();
        let found = m.search("run");
        let labels: Vec<&str> = found.iter().map(|i| m.rows[*i].label.as_str()).collect();
        assert!(labels.contains(&"Run by default"), "{labels:?}");
        assert!(labels.contains(&"Both buttons run forward"), "{labels:?}");
        assert!(m.search("  ").is_empty());
    }

    #[test]
    fn the_mixer_rows_grey_under_the_original_mixer() {
        let mut m = model();
        let voices = m
            .rows
            .iter()
            .find(|r| r.key == Some(OptionKey::Mixer(MixerField::VoiceCount)))
            .cloned()
            .expect("the row");
        m.set(
            OptionKey::Mixer(MixerField::RetailMixer),
            &OptionValue::Bool(true),
        );
        assert!(m.unavailable(&voices));
        m.set(
            OptionKey::Mixer(MixerField::RetailMixer),
            &OptionValue::Bool(false),
        );
        assert!(!m.unavailable(&voices));
    }
}
