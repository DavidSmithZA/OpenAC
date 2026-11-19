//! Which interface a client shows: the retail one, or OpenAC's modern one.
//!
//! The player picks it in the launcher, which remembers it and hands it to
//! every client it starts in [`INTERFACE_VARIABLE`]. The client reads the
//! variable once; a client started any other way shows the retail
//! interface.

/// The environment variable a launcher sets to an interface's
/// [`name`](Interface::name).
pub const INTERFACE_VARIABLE: &str = "OPENAC_INTERFACE";

/// The interface a client shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Interface {
    /// The retail interface, built from the dats' layouts.
    #[default]
    Retail,
    /// OpenAC's modern interface.
    Modern,
}

impl Interface {
    /// Every interface, in the order the launcher offers them.
    pub const ALL: [Self; 2] = [Self::Retail, Self::Modern];

    /// The name the variable and the launcher's settings file use.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Retail => "retail",
            Self::Modern => "modern",
        }
    }

    /// The interface `name` names (any case, surrounding space ignored).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim();
        Self::ALL
            .into_iter()
            .find(|interface| interface.name().eq_ignore_ascii_case(name))
    }

    /// The interface the launcher asked for (`read` looks up an
    /// environment variable); retail when it asked for none or for one this
    /// client does not know.
    pub fn requested(read: impl Fn(&str) -> Option<String>) -> Self {
        read(INTERFACE_VARIABLE)
            .and_then(|value| Self::from_name(&value))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_go_both_ways() {
        for interface in Interface::ALL {
            assert_eq!(Interface::from_name(interface.name()), Some(interface));
        }
        assert_eq!(Interface::from_name(" Modern "), Some(Interface::Modern));
        assert_eq!(Interface::from_name("enhanced"), None);
    }

    #[test]
    fn retail_unless_the_launcher_asks_for_modern() {
        let none = |_: &str| None;
        assert_eq!(Interface::requested(none), Interface::Retail);
        let unknown = |_: &str| Some("shiny".to_owned());
        assert_eq!(Interface::requested(unknown), Interface::Retail);
        let modern = |name: &str| (name == INTERFACE_VARIABLE).then(|| "modern".to_owned());
        assert_eq!(Interface::requested(modern), Interface::Modern);
    }
}
