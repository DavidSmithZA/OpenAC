//! What the Trade window shows, made from the trade the session sent, the
//! item table and what the window remembers: the two sides with their
//! items, acceptance and totals, the status, the banner when the other
//! player changed the trade after the player accepted (with what is new) or
//! when the game refused something, and the player's packs to add from.

use ac_core::items::{ClientObject, ClientObjectTable, ItemType};
use ac_runtime::inventory::TradeView;
use ac_runtime::inventory::shop::group_thousands;
use ac_ui::draw::{Color, TextureRef};

use super::vendor_model::{carried, category_of};

/// One item on a side.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeRow {
    /// The item.
    pub id: u32,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its name.
    pub name: String,
    /// What it is ("Jewelry · workmanship 6", "Coins · 2,500").
    pub sub: String,
    /// Its value.
    pub value: String,
    /// "exact" for money, "vendor value" for anything else.
    pub value_sub: &'static str,
    /// New since the player accepted.
    pub new: bool,
    /// A plugin's mark beside its name, in its colour.
    pub mark: Option<(String, Color)>,
}

/// One side of the trade.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TradeSide {
    /// "You offer" / "Osk offers".
    pub title: String,
    /// The first letter of the name.
    pub initial: String,
    /// Its items.
    pub rows: Vec<TradeRow>,
    /// It has accepted.
    pub accepted: bool,
    /// "2 items".
    pub count: String,
    /// "42,500p", or "vendor value 6,000p".
    pub total: String,
}

/// The banner over the sides.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeBanner {
    /// Its first line.
    pub title: String,
    /// Its second.
    pub text: String,
    /// Bad news (red), else a warning (gold).
    pub bad: bool,
}

/// One of the player's items in Add from packs.
#[derive(Debug, Clone, PartialEq)]
pub struct PickRow {
    /// The item.
    pub id: u32,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its name.
    pub name: String,
    /// Where it is and how many.
    pub sub: String,
    /// It cannot be given away: greyed, with the reason.
    pub refused: bool,
    /// Its stack (more than one asks how many).
    pub stack: i32,
}

/// What the Trade window shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TradeScreen {
    /// The other player's name.
    pub partner: String,
    /// The status pill's words, and whether it is good news, bad news or
    /// neither.
    pub status: String,
    pub status_tone: i8,
    /// The two sides: the player's, then the other player's.
    pub mine: TradeSide,
    pub theirs: TradeSide,
    /// The banner.
    pub banner: Option<TradeBanner>,
    /// What the player is told by the Accept button.
    pub hint: String,
    /// The other player has accepted (the hint is good news).
    pub hint_good: bool,
}

/// What the window remembers of the trade.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TradeMemory {
    /// The other player's items when the player last accepted (none: not
    /// accepted since the trade opened or was cleared).
    pub accepted_with: Option<Vec<u32>>,
    /// The acceptance was taken back by a change the player did not make.
    pub taken_back: bool,
    /// The last refusal: the item and the game's words.
    pub refused: Option<(String, String)>,
}

/// Whether `o` is money (its value is exact).
fn is_money(o: &ClientObject) -> bool {
    o.item_type
        .intersects(ItemType::MONEY | ItemType::PROMISSORY_NOTE)
}

fn row(
    objects: &ClientObjectTable,
    view: &TradeView,
    id: u32,
    new: bool,
    icon: &mut dyn FnMut(&ClientObject) -> Option<TextureRef>,
    names: &dyn Fn(&ClientObject) -> String,
    mark: &dyn Fn(u32) -> Option<(String, Color)>,
) -> (TradeRow, i64, bool) {
    let Some(o) = objects.get(id) else {
        let name = view
            .item_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| "Something".to_owned());
        return (
            TradeRow {
                id,
                icon: None,
                name,
                sub: String::new(),
                value: "\u{2014}".to_owned(),
                value_sub: "",
                new,
                mark: mark(id),
            },
            0,
            false,
        );
    };
    let kind = if is_money(o) {
        if o.item_type.intersects(ItemType::MONEY) {
            "Coins"
        } else {
            "Trade note"
        }
    } else {
        category_of(o.item_type.bits()).map_or("Item", |c| c.0)
    };
    let mut sub = kind.to_owned();
    if o.stack_size > 1 {
        sub.push_str(&format!(
            " \u{b7} {} in stack",
            group_thousands(o.stack_size)
        ));
    }
    if o.workmanship > 0.0 {
        sub.push_str(&format!(" \u{b7} workmanship {:.0}", o.workmanship));
    }
    let exact = is_money(o);
    (
        TradeRow {
            id,
            icon: icon(o),
            name: names(o),
            sub,
            value: format!("{}p", group_thousands(o.value)),
            value_sub: if exact { "exact" } else { "vendor value" },
            new,
            mark: mark(id),
        },
        i64::from(o.value),
        exact,
    )
}

fn count_words(n: usize) -> String {
    if n == 1 {
        "1 item".to_owned()
    } else {
        format!("{n} items")
    }
}

fn total_words(total: i64, exact: bool) -> String {
    let n = group_thousands(i32::try_from(total).unwrap_or(i32::MAX));
    if exact {
        format!("{n}p")
    } else {
        format!("vendor value {n}p")
    }
}

/// The first letter of `name`, as a capital.
fn initial(name: &str) -> String {
    name.chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default()
}

/// The Trade window's view of `view` with what the window remembers.
#[allow(clippy::too_many_arguments)]
pub fn screen(
    view: &TradeView,
    objects: &ClientObjectTable,
    me: &str,
    memory: &TradeMemory,
    icon: &mut dyn FnMut(&ClientObject) -> Option<TextureRef>,
    names: &dyn Fn(&ClientObject) -> String,
    mark: &dyn Fn(u32) -> Option<(String, Color)>,
) -> TradeScreen {
    let partner = view
        .partner_name
        .clone()
        .or_else(|| objects.get(view.partner_guid).map(|o| o.name.clone()))
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "The other player".to_owned());
    let side = |ids: &[u32],
                title: String,
                who: &str,
                accepted: bool,
                fresh: &dyn Fn(u32) -> bool,
                icon: &mut dyn FnMut(&ClientObject) -> Option<TextureRef>| {
        let mut total = 0i64;
        let mut exact = true;
        let rows: Vec<TradeRow> = ids
            .iter()
            .map(|&id| {
                let (r, value, money) = row(objects, view, id, fresh(id), icon, names, mark);
                total += value;
                exact &= money;
                r
            })
            .collect();
        TradeSide {
            title,
            initial: initial(who),
            count: count_words(rows.len()),
            total: total_words(total, exact),
            rows,
            accepted,
        }
    };
    let new = |id: u32| {
        memory.taken_back
            && memory
                .accepted_with
                .as_ref()
                .is_some_and(|was| !was.contains(&id))
    };
    let mine = side(
        &view.self_items,
        "You offer".to_owned(),
        me,
        view.self_accepted,
        &|_| false,
        icon,
    );
    let theirs = side(
        &view.partner_items,
        format!("{partner} offers"),
        &partner,
        view.partner_accepted,
        &new,
        icon,
    );
    // An acceptance shows over a refusal (the banner tells of that).
    let (status, status_tone) = match (view.self_accepted, view.partner_accepted) {
        (true, true) => ("Both accepted".to_owned(), 1),
        (true, false) => ("You accepted".to_owned(), 1),
        (false, true) => (format!("{partner} accepted"), 1),
        (false, false) if memory.refused.is_some() => ("Did not go through".to_owned(), -1),
        (false, false) => ("Offering".to_owned(), 0),
    };
    let banner = if let Some((item, words)) = &memory.refused {
        Some(TradeBanner {
            title: format!("{item} could not be traded."),
            text: words.clone(),
            bad: true,
        })
    } else if memory.taken_back {
        let fresh: Vec<String> = theirs
            .rows
            .iter()
            .filter(|r| r.new)
            .map(|r| r.name.clone())
            .collect();
        Some(TradeBanner {
            title: format!(
                "{partner} changed the trade after you accepted, so your acceptance was taken back."
            ),
            text: if fresh.is_empty() {
                "Look again before you accept.".to_owned()
            } else {
                format!(
                    "New since you accepted: {}. Look again before you accept.",
                    fresh.join(", ")
                )
            },
            bad: false,
        })
    } else {
        None
    };
    let (hint, hint_good) = if view.partner_accepted {
        (
            format!("{partner} accepted. Your accept completes the trade."),
            true,
        )
    } else {
        (format!("{partner} has not accepted yet."), false)
    };
    TradeScreen {
        partner,
        status,
        status_tone,
        mine,
        theirs,
        banner,
        hint,
        hint_good,
    }
}

/// The player's items Add from packs lists: everything carried and not
/// worn, not already offered; attuned items greyed.
#[must_use]
pub fn pick_rows(
    objects: &ClientObjectTable,
    player: u32,
    offered: &[u32],
    search: &str,
    icon: &mut dyn FnMut(&ClientObject) -> Option<TextureRef>,
    names: &dyn Fn(&ClientObject) -> String,
) -> Vec<PickRow> {
    let s = search.trim().to_lowercase();
    carried(objects, player)
        .into_iter()
        .filter(|(id, _)| !offered.contains(id))
        .filter_map(|(id, pack)| {
            let o = objects.get(id)?;
            let name = names(o);
            if !s.is_empty() && !name.to_lowercase().contains(&s) {
                return None;
            }
            let place = if pack == player {
                "Main pack".to_owned()
            } else {
                objects
                    .get(pack)
                    .map_or_else(|| "Pack".to_owned(), |p| p.appropriate_name().into_owned())
            };
            let stack = o.stack_size.max(1);
            let sub = if o.attuned {
                "Attuned: it cannot be given away".to_owned()
            } else if stack > 1 {
                format!("{place} \u{b7} {} in stack", group_thousands(stack))
            } else {
                place
            };
            Some(PickRow {
                id,
                icon: icon(o),
                name,
                sub,
                refused: o.attuned,
                stack,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_core::items::EquipMask;

    const ME: u32 = 0x5000_0001;
    const OSK: u32 = 0x5000_0002;

    fn put(t: &mut ClientObjectTable, o: ClientObject, container: u32) {
        let id = o.object_id();
        let mut ev = Vec::new();
        t.add_or_update(o, &mut ev);
        if container != 0 {
            t.move_item(id, container, -1, EquipMask::NONE, None, &mut ev);
        }
    }

    fn table() -> ClientObjectTable {
        let mut t = ClientObjectTable::new();
        let mut me = ClientObject::new(ME);
        me.name = "Kael".into();
        me.items_capacity = 102;
        put(&mut t, me, 0);
        let mut note = ClientObject::new(0x8000_0001);
        note.name = "Trade Note (20,000)".into();
        note.item_type = ItemType::PROMISSORY_NOTE;
        note.value = 40_000;
        note.stack_size = 2;
        put(&mut t, note, ME);
        let mut sword = ClientObject::new(0x8000_0002);
        sword.name = "Heirloom Sword".into();
        sword.item_type = ItemType::MELEE_WEAPON;
        sword.attuned = true;
        put(&mut t, sword, ME);
        let mut ring = ClientObject::new(0x8000_0010);
        ring.name = "Silver Ring".into();
        ring.item_type = ItemType::JEWELRY;
        ring.value = 6_000;
        ring.workmanship = 6.0;
        put(&mut t, ring, 0);
        let mut amber = ClientObject::new(0x8000_0011);
        amber.name = "Amber".into();
        amber.item_type = ItemType::GEM;
        amber.value = 360;
        put(&mut t, amber, 0);
        t
    }

    fn trade() -> TradeView {
        TradeView {
            open: true,
            partner_guid: OSK,
            partner_name: Some("Osk".into()),
            self_items: vec![0x8000_0001],
            partner_items: vec![0x8000_0010, 0x8000_0011],
            ..TradeView::default()
        }
    }

    fn show(view: &TradeView, memory: &TradeMemory) -> TradeScreen {
        let t = table();
        screen(
            view,
            &t,
            "Kael",
            memory,
            &mut |_| None,
            &|o| o.name.clone(),
            &|_| None,
        )
    }

    #[test]
    fn money_is_exact_and_anything_else_is_at_vendor_value() {
        let s = show(&trade(), &TradeMemory::default());
        assert_eq!(s.mine.total, "40,000p");
        assert_eq!(s.theirs.total, "vendor value 6,360p");
        assert_eq!(s.theirs.title, "Osk offers");
        assert_eq!(s.theirs.initial, "O");
        assert_eq!(s.status, "Offering");
    }

    #[test]
    fn a_change_after_accepting_marks_what_is_new() {
        let memory = TradeMemory {
            accepted_with: Some(vec![0x8000_0010]),
            taken_back: true,
            refused: None,
        };
        let s = show(&trade(), &memory);
        let new: Vec<bool> = s.theirs.rows.iter().map(|r| r.new).collect();
        assert_eq!(new, [false, true]);
        let b = s.banner.expect("the banner");
        assert!(b.text.contains("Amber"), "{}", b.text);
        assert!(!b.bad);
    }

    #[test]
    fn the_status_follows_who_accepted() {
        let mut v = trade();
        v.partner_accepted = true;
        let s = show(&v, &TradeMemory::default());
        assert_eq!(s.status, "Osk accepted");
        assert!(s.hint_good);
        v.self_accepted = true;
        assert_eq!(show(&v, &TradeMemory::default()).status, "Both accepted");
    }

    #[test]
    fn offered_items_leave_the_pick_list_and_attuned_ones_are_greyed() {
        let t = table();
        let rows = pick_rows(&t, ME, &[0x8000_0001], "", &mut |_| None, &|o| {
            o.name.clone()
        });
        assert_eq!(rows.len(), 1);
        assert!(rows[0].refused);
        assert!(rows[0].sub.starts_with("Attuned"));
    }
}
