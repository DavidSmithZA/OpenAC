//! What the modern Options window lists: eight sections, each a run of
//! groups, each group a run of settings in the window's own words. What a
//! setting is (switch, slider, choice), its range, choices and default
//! come from the game's option pages (`ac_ui::panels::options::spec`), so
//! the two windows never disagree; [`LEFT_OUT`] names every setting of
//! those pages this window does not list, and why.

use ac_runtime::character::option_id as o;
use ac_ui::panels::options::keys::{
    AudioField, CameraField, ChatField, DisplayField, MixerField, OptionKey, RenderPackField,
};

/// A section of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    /// Scale, HUD, vitals, world effects.
    Interface,
    /// Combat, items, trading.
    Gameplay,
    /// Fellowship, allegiance, what others see.
    Social,
    /// Channels, the tabs' message types, behaviour.
    Chat,
    /// Mouse and keys.
    Controls,
    /// Display, quality, performance, atmosphere.
    Graphics,
    /// Volume and the mixer.
    Sound,
    /// The camera.
    Camera,
}

impl Section {
    /// Every section, in the list's order.
    pub const ALL: [Self; 8] = [
        Self::Interface,
        Self::Gameplay,
        Self::Social,
        Self::Chat,
        Self::Controls,
        Self::Graphics,
        Self::Sound,
        Self::Camera,
    ];

    /// Its name.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::Interface => "Interface",
            Self::Gameplay => "Gameplay",
            Self::Social => "Social",
            Self::Chat => "Chat",
            Self::Controls => "Controls",
            Self::Graphics => "Graphics",
            Self::Sound => "Sound",
            Self::Camera => "Camera",
        }
    }

    /// The line under its title.
    #[must_use]
    pub const fn intro(self) -> &'static str {
        match self {
            Self::Interface => "How the game looks around the world: size, HUD and effects.",
            Self::Gameplay => "How your character fights and handles items.",
            Self::Social => "Fellowships, allegiance and what other players can see about you.",
            Self::Chat => "Which channels you hear and what each chat tab shows.",
            Self::Controls => "Mouse and keyboard. Click a key to change it.",
            Self::Graphics => "Screen, detail and the extra effects OpenAC can draw.",
            Self::Sound => "Volume, and how sounds are mixed.",
            Self::Camera => "How the camera follows you.",
        }
    }

    /// Its glyph: polylines in a 24 unit box.
    #[must_use]
    pub const fn glyph(self) -> &'static [&'static [(f32, f32)]] {
        match self {
            Self::Interface => &[
                &[
                    (3.0, 4.0),
                    (21.0, 4.0),
                    (21.0, 16.0),
                    (3.0, 16.0),
                    (3.0, 4.0),
                ],
                &[(8.0, 20.0), (16.0, 20.0)],
                &[(12.0, 16.0), (12.0, 20.0)],
            ],
            Self::Gameplay => &[
                &[
                    (14.5, 17.5),
                    (3.0, 6.0),
                    (3.0, 3.0),
                    (6.0, 3.0),
                    (17.5, 14.5),
                ],
                &[(13.0, 19.0), (19.0, 13.0)],
                &[(16.0, 16.0), (20.0, 20.0)],
            ],
            Self::Social => &[
                &[
                    (9.0, 3.5),
                    (12.5, 5.0),
                    (13.0, 8.5),
                    (11.0, 11.0),
                    (7.0, 11.0),
                    (5.0, 8.5),
                    (5.5, 5.0),
                    (9.0, 3.5),
                ],
                &[
                    (2.0, 21.0),
                    (4.0, 16.0),
                    (9.0, 14.0),
                    (14.0, 16.0),
                    (16.0, 21.0),
                ],
                &[(16.0, 3.5), (19.5, 5.5), (19.5, 9.0), (16.0, 11.0)],
                &[(18.0, 14.5), (21.0, 17.0), (22.0, 21.0)],
            ],
            Self::Chat => &[&[
                (21.0, 12.0),
                (18.5, 17.5),
                (12.0, 20.0),
                (8.5, 19.0),
                (3.0, 21.0),
                (4.9, 15.0),
                (4.0, 12.0),
                (6.5, 6.5),
                (12.0, 4.0),
                (18.5, 6.5),
                (21.0, 12.0),
            ]],
            Self::Controls => &[
                &[
                    (3.0, 7.0),
                    (21.0, 7.0),
                    (21.0, 17.0),
                    (3.0, 17.0),
                    (3.0, 7.0),
                ],
                &[(7.0, 11.0), (7.5, 11.0)],
                &[(11.0, 11.0), (11.5, 11.0)],
                &[(15.0, 11.0), (15.5, 11.0)],
                &[(7.0, 14.0), (17.0, 14.0)],
            ],
            Self::Graphics => &[
                &[
                    (3.0, 5.0),
                    (21.0, 5.0),
                    (21.0, 19.0),
                    (3.0, 19.0),
                    (3.0, 5.0),
                ],
                &[
                    (3.0, 15.0),
                    (8.0, 10.0),
                    (12.0, 14.0),
                    (15.0, 11.0),
                    (21.0, 17.0),
                ],
            ],
            Self::Sound => &[
                &[
                    (11.0, 5.0),
                    (6.0, 9.0),
                    (2.0, 9.0),
                    (2.0, 15.0),
                    (6.0, 15.0),
                    (11.0, 19.0),
                    (11.0, 5.0),
                ],
                &[(15.5, 8.5), (17.0, 12.0), (15.5, 15.5)],
                &[(19.0, 5.0), (22.0, 12.0), (19.0, 19.0)],
            ],
            Self::Camera => &[
                &[
                    (3.0, 7.0),
                    (7.0, 7.0),
                    (9.0, 4.0),
                    (15.0, 4.0),
                    (17.0, 7.0),
                    (21.0, 7.0),
                    (21.0, 20.0),
                    (3.0, 20.0),
                    (3.0, 7.0),
                ],
                &[
                    (12.0, 9.0),
                    (15.0, 10.5),
                    (16.0, 13.0),
                    (15.0, 15.5),
                    (12.0, 17.0),
                    (9.0, 15.5),
                    (8.0, 13.0),
                    (9.0, 10.5),
                    (12.0, 9.0),
                ],
            ],
        }
    }
}

/// What a setting is called and what it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Words {
    /// Its name.
    pub label: &'static str,
    /// A line on what it does (may be empty).
    pub desc: &'static str,
    /// The switch shows the opposite of the stored value (the game's
    /// "Disable fog" shows as "Distance fog", on while fog shows).
    pub invert: bool,
}

const fn w(label: &'static str, desc: &'static str) -> Words {
    Words {
        label,
        desc,
        invert: false,
    }
}

const fn not(label: &'static str, desc: &'static str) -> Words {
    Words {
        label,
        desc,
        invert: true,
    }
}

/// A button's action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// The shortcut bars' edit layout mode.
    EditBars,
    /// The game's mouse turning settings, all at once.
    MouseTurning,
    /// Load a key profile.
    LoadKeys,
    /// Save the keys as a profile.
    SaveKeysAs,
}

/// One entry of a group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Item {
    /// A setting.
    Key(OptionKey, Words),
    /// A switch and its volume on one line: the switch's key, the volume's.
    Volume(OptionKey, OptionKey, Words),
    /// The atmospheric rendering's own settings (from its table).
    Atmospheric,
    /// The chat tabs against the message types.
    ChatGrid,
    /// A button.
    Button(Action, Words, &'static str),
}

/// A group of a section.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Group {
    /// The section it is in.
    pub section: Section,
    /// Its heading (spaced capitals).
    pub title: &'static str,
    /// Its entries.
    pub items: &'static [Item],
}

const fn ch(id: u32, words: Words) -> Item {
    Item::Key(OptionKey::Character(id), words)
}

const fn disp(f: DisplayField, words: Words) -> Item {
    Item::Key(OptionKey::Display(f), words)
}

const fn cam(f: CameraField, words: Words) -> Item {
    Item::Key(OptionKey::Camera(f), words)
}

const fn mix(f: MixerField, words: Words) -> Item {
    Item::Key(OptionKey::Mixer(f), words)
}

/// Every group, by section, in the window's order.
pub const GROUPS: &[Group] = &[
    // Interface.
    Group {
        section: Section::Interface,
        title: "SIZE",
        items: &[disp(
            DisplayField::UiScale,
            w(
                "UI scale",
                "Windows, text and controls together. The world is not scaled.",
            ),
        )],
    },
    Group {
        section: Section::Interface,
        title: "SHORTCUT BARS",
        items: &[Item::Button(
            Action::EditBars,
            w(
                "Shortcut bars",
                "Move, turn and resize your bars, and set the key for each slot.",
            ),
            "Edit layout",
        )],
    },
    Group {
        section: Section::Interface,
        title: "HUD",
        items: &[
            ch(
                o::COORDINATES_ON_RADAR,
                w(
                    "Coordinates on radar",
                    "Your map coordinates under the radar.",
                ),
            ),
            ch(
                o::SPELL_DURATION,
                w(
                    "Show spell durations",
                    "Time left on helpful and harmful effects.",
                ),
            ),
            ch(
                o::VIVID_TARGETING_INDICATOR,
                w(
                    "Vivid targeting indicator",
                    "A brighter marker on your current target.",
                ),
            ),
            ch(
                o::SHOW_TOOLTIPS,
                w(
                    "Tooltips",
                    "Explain buttons and items when you hover over them.",
                ),
            ),
        ],
    },
    Group {
        section: Section::Interface,
        title: "VITALS AND EFFECTS",
        items: &[
            disp(
                DisplayField::ModernAnimateVitals,
                w(
                    "Animate the vital bars",
                    "Losses flash and drain away, gains fill in behind a bright edge.",
                ),
            ),
            disp(
                DisplayField::ModernVitalNumbers,
                w(
                    "Damage and healing numbers",
                    "A number falls off the bar for changes at least this big. Small hits and regeneration stay quiet.",
                ),
            ),
            disp(
                DisplayField::ModernLowHealth,
                w("Low health warning", "The health bar pulses below this."),
            ),
            disp(
                DisplayField::ModernEdgeGlow,
                w(
                    "Glow the screen edge at low health",
                    "A soft red pulse around the edge of the screen.",
                ),
            ),
            disp(
                DisplayField::ModernEffectWarning,
                w(
                    "Warn before a helpful effect ends",
                    "Its count turns amber this long before.",
                ),
            ),
        ],
    },
    Group {
        section: Section::Interface,
        title: "WORLD EFFECTS",
        items: &[
            ch(
                o::DISABLE_MOST_WEATHER_EFFECTS,
                not("Most weather effects", "Rain, snow and lightning."),
            ),
            ch(
                o::DISABLE_DISTANCE_FOG,
                not("Distance fog", "Haze that hides the far landscape."),
            ),
            ch(
                o::PERSISTENT_AT_DAY,
                w("Always daytime", "Keep the sky at day."),
            ),
            ch(
                o::DISABLE_HOUSE_RESTRICTION_EFFECTS,
                not(
                    "House restriction effects",
                    "The glow around housing you may not enter.",
                ),
            ),
        ],
    },
    // Gameplay.
    Group {
        section: Section::Gameplay,
        title: "COMBAT",
        items: &[
            ch(
                o::AUTO_TARGET,
                w(
                    "Auto target",
                    "Pick the nearest enemy when you attack with nothing selected.",
                ),
            ),
            ch(
                o::AUTO_REPEAT_ATTACK,
                w(
                    "Repeat attacks",
                    "Keep attacking until you stop or your target dies.",
                ),
            ),
            ch(
                o::VIEW_COMBAT_TARGET,
                w(
                    "Keep target in view",
                    "Turn the camera to keep your target on screen.",
                ),
            ),
            ch(
                o::USE_CHARGE_ATTACK,
                w("Charge attacks", "Hold the attack key to build up power."),
            ),
            ch(
                o::LEAD_MISSILE_TARGETS,
                w(
                    "Lead moving targets",
                    "Aim missiles where a moving target will be.",
                ),
            ),
            ch(
                o::USE_FAST_MISSILES,
                w(
                    "Fast missiles",
                    "Skip the draw animation where the server allows.",
                ),
            ),
            ch(
                o::ADVANCED_COMBAT_UI,
                w(
                    "Advanced combat controls",
                    "Separate attack height and power controls.",
                ),
            ),
            ch(
                o::TOGGLE_RUN,
                w("Run by default", "Run without holding the run key."),
            ),
        ],
    },
    Group {
        section: Section::Gameplay,
        title: "ITEMS",
        items: &[
            ch(
                o::SALVAGE_MULTIPLE,
                w(
                    "Salvage several items at once",
                    "Put more than one item in the salvage window.",
                ),
            ),
            ch(
                o::MAIN_PACK_PREFERRED,
                w(
                    "New items go to the main pack",
                    "Instead of the first side pack with room.",
                ),
            ),
            ch(
                o::CONFIRM_VOLATILE_RARE_USE,
                w(
                    "Confirm using volatile rares",
                    "Ask before using a rare that is used up.",
                ),
            ),
            ch(
                o::USE_CRAFT_SUCCESS_DIALOG,
                w(
                    "Show craft success chance",
                    "Ask with the chance before tinkering or crafting.",
                ),
            ),
        ],
    },
    Group {
        section: Section::Gameplay,
        title: "TRADING",
        items: &[
            ch(o::ALLOW_GIVE, w("Let others give you items", "")),
            ch(o::IGNORE_TRADE_REQUESTS, w("Ignore trade requests", "")),
            ch(
                o::DRAG_ITEM_ON_PLAYER_OPENS_SECURE_TRADE,
                w(
                    "Dropping an item on a player opens trade",
                    "Instead of giving it.",
                ),
            ),
            ch(
                o::ACCEPT_LOOT_PERMITS,
                w(
                    "Accept loot permits",
                    "Let others allow you to loot their kills.",
                ),
            ),
            ch(
                o::USE_DECEPTION,
                w(
                    "Use deception",
                    "Your Deception skill works against players who assess you.",
                ),
            ),
        ],
    },
    // Social.
    Group {
        section: Section::Social,
        title: "FELLOWSHIP",
        items: &[
            ch(o::FELLOWSHIP_SHARE_XP, w("Share experience", "")),
            ch(o::FELLOWSHIP_SHARE_LOOT, w("Share loot", "")),
            ch(
                o::FELLOWSHIP_AUTO_ACCEPT_REQUESTS,
                w("Accept invitations automatically", ""),
            ),
            ch(o::IGNORE_FELLOWSHIP_REQUESTS, w("Ignore invitations", "")),
        ],
    },
    Group {
        section: Section::Social,
        title: "ALLEGIANCE",
        items: &[
            ch(
                o::IGNORE_ALLEGIANCE_REQUESTS,
                w("Ignore swear requests", ""),
            ),
            ch(
                o::DISPLAY_ALLEGIANCE_LOGON_NOTIFICATIONS,
                w("Logon notices", "Tell me when allegiance members log on."),
            ),
        ],
    },
    Group {
        section: Section::Social,
        title: "WHAT OTHERS SEE",
        items: &[
            ch(o::SHOW_HELM, w("Your helm", "")),
            ch(o::SHOW_CLOAK, w("Your cloak", "")),
            ch(o::DISPLAY_DATE_OF_BIRTH, w("Date of birth", "")),
            ch(o::DISPLAY_AGE, w("Age", "")),
            ch(o::DISPLAY_CHESS_RANK, w("Chess rank", "")),
            ch(o::DISPLAY_FISHING_SKILL, w("Fishing skill", "")),
            ch(o::DISPLAY_NUMBER_DEATHS, w("Number of deaths", "")),
            ch(
                o::DISPLAY_NUMBER_CHARACTER_TITLES,
                w("Number of titles", ""),
            ),
        ],
    },
    // Chat.
    Group {
        section: Section::Chat,
        title: "CHANNELS YOU HEAR",
        items: &[
            ch(o::LISTEN_TO_GENERAL_CHAT, w("General", "")),
            ch(o::LISTEN_TO_TRADE_CHAT, w("Trade", "")),
            ch(o::LISTEN_TO_LFG_CHAT, w("Looking for group", "")),
            ch(o::LISTEN_TO_ROLEPLAY_CHAT, w("Roleplay", "")),
            ch(o::LISTEN_TO_SOCIETY_CHAT, w("Society", "")),
            ch(o::LISTEN_TO_ALLEGIANCE_CHAT, w("Allegiance", "")),
            ch(o::HEAR_PK_DEATH_MESSAGES, w("Player killer deaths", "")),
        ],
    },
    Group {
        section: Section::Chat,
        title: "WHAT EACH TAB SHOWS (SAVED AT ONCE)",
        items: &[Item::ChatGrid],
    },
    Group {
        section: Section::Chat,
        title: "BEHAVIOUR",
        items: &[
            ch(
                o::DISPLAY_TIME_STAMPS,
                w("Timestamps", "Show the time beside each line."),
            ),
            ch(
                o::STAY_IN_CHAT_MODE,
                w(
                    "Stay in chat after sending",
                    "Keep typing without pressing Enter again.",
                ),
            ),
            ch(o::FILTER_LANGUAGE, w("Filter bad language", "")),
        ],
    },
    // Controls.
    Group {
        section: Section::Controls,
        title: "MOUSE",
        items: &[
            cam(
                CameraField::ModernMouseTurning,
                w(
                    "Modern mouse turning",
                    "Hold the right button and drag: the view turns, and your character turns to face it.",
                ),
            ),
            cam(
                CameraField::BothMouseButtonsRunForward,
                w(
                    "Both buttons run forward",
                    "With modern mouse turning: hold both mouse buttons to run.",
                ),
            ),
            cam(
                CameraField::UseMouseTurning,
                w(
                    "Mouse turning",
                    "The game's own: the right button turns your character. Modern mouse turning replaces it.",
                ),
            ),
            Item::Button(
                Action::MouseTurning,
                w(
                    "The game's mouse turning settings",
                    "Camera and mouse set as the game suggests for mouse turning.",
                ),
                "Use them",
            ),
            cam(CameraField::MouseLookSensitivity, w("Mouse look speed", "")),
            cam(
                CameraField::InvertMouseLookYAxis,
                w("Invert mouse look", "Moving the mouse up looks down."),
            ),
        ],
    },
    Group {
        section: Section::Controls,
        title: "QUICK MOVE",
        items: &[disp(
            DisplayField::ModernQuickMoveKey,
            w(
                "Quick move key",
                "Hold it and drag any piece of the interface anywhere; with a right-click, the piece's menu. On a Mac, Control with a click is often a right-click, so Option or Command may suit you better.",
            ),
        )],
    },
    Group {
        section: Section::Controls,
        title: "KEY PROFILES",
        items: &[
            Item::Button(
                Action::LoadKeys,
                w(
                    "Load a key profile",
                    "Every key from a profile you saved before.",
                ),
                "Load",
            ),
            Item::Button(
                Action::SaveKeysAs,
                w("Save these keys", "As a profile of their own."),
                "Save as",
            ),
        ],
    },
    // Graphics.
    Group {
        section: Section::Graphics,
        title: "DISPLAY",
        items: &[
            disp(DisplayField::Resolution, w("Resolution", "")),
            disp(
                DisplayField::Fullscreen,
                w("Full screen", "Off: the game runs in a window."),
            ),
            disp(
                DisplayField::VSync,
                w(
                    "Sync to display",
                    "Stops tearing. Caps the frame rate at the display rate.",
                ),
            ),
            disp(DisplayField::ScreenBrightness, w("Brightness", "")),
            disp(
                DisplayField::ModernGlassBlur,
                w(
                    "Frosted glass",
                    "Windows and pop-ups blur the world behind them. Off: a plain see-through fill, a little lighter on the graphics card.",
                ),
            ),
        ],
    },
    Group {
        section: Section::Graphics,
        title: "QUALITY",
        items: &[
            disp(
                DisplayField::Quality,
                w("Graphics profile", "Sets the detail of the world at once."),
            ),
            disp(
                DisplayField::PotatoMode,
                w(
                    "Potato mode",
                    "Everything at its cheapest, for many clients on one machine.",
                ),
            ),
            disp(
                DisplayField::LandscapeDrawDistance,
                w("Draw distance", "How far the landscape reaches."),
            ),
            disp(
                DisplayField::LandscapeTextureDetail,
                w("Landscape textures", ""),
            ),
            disp(
                DisplayField::EnvironmentTextureDetail,
                w("Object textures", ""),
            ),
            disp(DisplayField::TextureFiltering, w("Texture filtering", "")),
            disp(
                DisplayField::BuildingDetailTextures,
                w("Building detail textures", ""),
            ),
            disp(
                DisplayField::LandscapeDetailTextures,
                w("Landscape detail textures", ""),
            ),
            disp(DisplayField::MultiPassAlpha, w("Multi-pass alpha", "")),
            disp(
                DisplayField::KeepDistantBuildings,
                w(
                    "Keep distant buildings",
                    "Far buildings keep their simplest shape instead of disappearing.",
                ),
            ),
        ],
    },
    Group {
        section: Section::Graphics,
        title: "PERFORMANCE",
        items: &[
            disp(
                DisplayField::AutomaticDegrades,
                w(
                    "Lower detail when busy",
                    "Detail drops while the frame rate does.",
                ),
            ),
            disp(DisplayField::GraphicsPerformance, w("Speed or detail", "")),
            disp(DisplayField::DegradeDistance, w("Full detail distance", "")),
            disp(
                DisplayField::UiOnly,
                w(
                    "Draw only the interface",
                    "Stop drawing the world; chat, panels and plugins keep working.",
                ),
            ),
            disp(
                DisplayField::UiOnlyWhenUnfocused,
                w(
                    "Draw only the active window",
                    "Other OpenAC windows keep their chat and plugins but stop drawing the world.",
                ),
            ),
        ],
    },
    Group {
        section: Section::Graphics,
        title: "ATMOSPHERIC RENDERING",
        items: &[
            Item::Key(
                OptionKey::RenderPack(RenderPackField::Quality),
                w(
                    "Atmospheric rendering",
                    "Light, shadows and air. Costs the most of any setting.",
                ),
            ),
            Item::Atmospheric,
        ],
    },
    // Sound.
    Group {
        section: Section::Sound,
        title: "VOLUME",
        items: &[
            Item::Volume(
                OptionKey::Audio(AudioField::SfxEnabled),
                OptionKey::Audio(AudioField::Sfx),
                w("Effects", ""),
            ),
            Item::Volume(
                OptionKey::Audio(AudioField::AmbientEnabled),
                OptionKey::Audio(AudioField::Ambient),
                w("Ambient", ""),
            ),
            Item::Volume(
                OptionKey::Audio(AudioField::InterfaceEnabled),
                OptionKey::Audio(AudioField::InterfaceVolume),
                w("Interface", ""),
            ),
            Item::Key(
                OptionKey::Audio(AudioField::PlaySoundOnlyWhenActive),
                w(
                    "Mute in the background",
                    "Silent while another window is active.",
                ),
            ),
            Item::Key(
                OptionKey::Audio(AudioField::SoundFeatures),
                w("Speakers", ""),
            ),
        ],
    },
    Group {
        section: Section::Sound,
        title: "MIXER",
        items: &[
            mix(
                MixerField::RetailMixer,
                w(
                    "Mix as the original client",
                    "16 voices, no priority, no cap per sound. Overrides the settings below.",
                ),
            ),
            mix(
                MixerField::VoiceCount,
                w("Voices", "Sounds playing at once."),
            ),
            mix(
                MixerField::UseAuthoredPriority,
                w(
                    "Priority",
                    "An important sound may take the voice of a quieter one.",
                ),
            ),
            mix(
                MixerField::MaxVoicesPerWave,
                w("Voices per sound", "How often one sound can play at once."),
            ),
        ],
    },
    // Camera.
    Group {
        section: Section::Camera,
        title: "CAMERA",
        items: &[
            cam(
                CameraField::Stiffness,
                w("Stiffness", "How tightly the camera follows you."),
            ),
            cam(
                CameraField::AdjustmentSpeed,
                w("Adjustment speed", "How fast it settles behind you."),
            ),
            disp(DisplayField::FieldOfView, w("Field of view", "")),
            cam(
                CameraField::AlignToSlope,
                w("Align to slope", "Tilt with the ground you stand on."),
            ),
        ],
    },
];

/// The game's settings this window does not list, and why.
pub const LEFT_OUT: &[(OptionKey, &str)] = &[
    (
        OptionKey::Character(o::SIDE_BY_SIDE_VITALS),
        "the modern vitals have one layout",
    ),
    (
        OptionKey::Chat(ChatField::FontFace),
        "the modern chat has its own typeface",
    ),
    (
        OptionKey::Chat(ChatField::FontSize),
        "the modern chat has its own typeface",
    ),
    (
        OptionKey::Chat(ChatField::DefaultOpacity),
        "each modern chat window has its own shading",
    ),
    (
        OptionKey::Chat(ChatField::ActiveOpacity),
        "each modern chat window has its own shading",
    ),
];

/// A retail chat window's filter: the modern chat's tabs replace those
/// windows ([`Item::ChatGrid`]).
#[must_use]
pub const fn is_retail_chat_filter(key: OptionKey) -> bool {
    matches!(key, OptionKey::Chat(ChatField::Filter(_)))
}
