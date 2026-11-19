//! The player's own shortcut bars: as many as they like, anywhere on the
//! screen, across or upright, items and spells on the same bars, a key for
//! each slot outside Magic and another in Magic. Spells are greyed and do
//! nothing outside Magic; an item works by click in any mode. Kept on this
//! computer, per character or shared by the characters that ask for it;
//! the first time a character plays with them they are made from its
//! retail toolbar and favourite spells (the server's lists stay retail's).

pub mod keys;
pub mod model;
