//! What the Vendor window shows, made from the shop the session sent, the
//! player's items and what the player picked in the window: the Buy tab's
//! categories and goods, the Sell tab's packs and the player's own items
//! (each with what the vendor pays, or why it will not), the chosen thing
//! with its quantity and price, the To buy or To sell list with its total,
//! and the spell components the player is short of.

use std::collections::BTreeMap;

use ac_core::items::component_fill::{ANY_CATEGORY, build_component_desires, plan_component_fill};
use ac_core::items::placement::{is_pack, loose_contents};
use ac_core::items::vendor::{
    NO_LIMIT, UNLIMITED_STOCK, buy_price, per_unit_value, resolve_authored_stack_size,
};
use ac_core::items::{
    ClientObject, ClientObjectTable, EquipMask, VendorSellRejection, VendorShopItem,
    VendorStagingEntry,
};
use ac_runtime::inventory::VendorView;
use ac_runtime::inventory::shop::{group_thousands, has_room_in, listing_price};
use ac_ui::draw::TextureRef;
use ac_ui::panels::items::ItemsView;
use ac_ui::panels::vendor::CATEGORY_FILTERS;
use ac_ui::panels::vendor_rules::sell_check;

/// The window's two tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VendorTab {
    /// What the shop sells.
    #[default]
    Buy,
    /// What the player carries.
    Sell,
}

/// How the list is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VendorSort {
    /// By category (goods) or pack (the player's), then name.
    #[default]
    Group,
    /// By name.
    Name,
    /// Dearest first.
    Price,
}

impl VendorSort {
    /// The next order, as the sort button steps through them.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Group => Self::Name,
            Self::Name => Self::Price,
            Self::Price => Self::Group,
        }
    }

    /// Its name on the button.
    #[must_use]
    pub fn words(self, tab: VendorTab) -> &'static str {
        match (self, tab) {
            (Self::Group, VendorTab::Buy) => "Category",
            (Self::Group, VendorTab::Sell) => "Pack",
            (Self::Name, _) => "Name",
            (Self::Price, VendorTab::Buy) => "Price",
            (Self::Price, VendorTab::Sell) => "Value",
        }
    }
}

/// What the player picked in the window.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VendorChoices {
    /// The tab shown.
    pub tab: VendorTab,
    /// The Buy tab's category (its mask; 0: all goods).
    pub category: u32,
    /// The Sell tab's pack (0: all packs).
    pub pack: u32,
    /// The Sell tab hides what the vendor will not take.
    pub only_taken: bool,
    /// The search.
    pub search: String,
    /// The order.
    pub sort: VendorSort,
    /// The chosen listing or item.
    pub chosen: Option<u32>,
    /// How many of it (none: the usual: one to buy, the whole stack to
    /// sell).
    pub quantity: Option<i32>,
}

/// A row of the rail down the left.
#[derive(Debug, Clone, PartialEq)]
pub struct RailRow {
    /// Its words.
    pub words: String,
    /// What picking it picks (a category's mask, a pack; 0: all).
    pub key: u32,
    /// How many rows it shows.
    pub count: usize,
}

/// A row of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct VendorRow {
    /// The listing or item.
    pub id: u32,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its name.
    pub name: String,
    /// The line under it.
    pub sub: String,
    /// That line is a refusal, or a shortfall.
    pub sub_tone: Tone,
    /// "On list ×2" / "Selling ×3" (none: not on the list).
    pub on_list: Option<String>,
    /// The price ("—" for one the vendor will not take).
    pub price: String,
    /// The words under the price ("each", "they pay each").
    pub price_sub: String,
    /// The vendor will not take it: drawn faint.
    pub refused: bool,
    /// Its stack (the Sell tab).
    pub stack: i32,
    /// A plugin's mark beside its name, in its colour.
    pub mark: Option<(String, ac_ui::draw::Color)>,
}

/// How a line reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    /// Quiet.
    #[default]
    Quiet,
    /// Good news (what the player receives).
    Good,
    /// A warning (short of something).
    Warn,
    /// Bad news (a refusal, too dear, no room).
    Bad,
}

/// The chosen listing or item, with its quantity.
#[derive(Debug, Clone, PartialEq)]
pub struct VendorChosen {
    /// The listing or item.
    pub id: u32,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its name.
    pub name: String,
    /// The line under it.
    pub sub: String,
    /// How many.
    pub quantity: i32,
    /// The most there can be.
    pub most: i32,
    /// The Max button's words ("Max", "All", "To restock").
    pub most_words: &'static str,
    /// What the quantity comes to.
    pub total: String,
    /// The total's tone (green for what the player receives).
    pub total_tone: Tone,
    /// A line under the total.
    pub note: String,
    /// Its tone.
    pub note_tone: Tone,
    /// The main button's words ("Buy now", "Sell now").
    pub primary: &'static str,
    /// The main button works.
    pub can_primary: bool,
    /// Add to list works.
    pub can_add: bool,
}

/// A row of the To buy or To sell list.
#[derive(Debug, Clone, PartialEq)]
pub struct CartRow {
    /// The listing or item.
    pub id: u32,
    /// Its icon.
    pub icon: Option<TextureRef>,
    /// Its name.
    pub name: String,
    /// How many.
    pub quantity: i32,
    /// What the row comes to.
    pub price: String,
}

/// The spell components the player carries fewer of than the restock
/// amounts set in the Spellbook, and what buying them all costs.
#[derive(Debug, Clone, PartialEq)]
pub struct Restock {
    /// How many components are short.
    pub count: usize,
    /// What they cost together.
    pub cost: String,
    /// The listings and amounts to put on the buy list.
    pub adds: Vec<(u32, i32)>,
}

/// What the Vendor window shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VendorScreen {
    /// The vendor's name.
    pub name: String,
    /// The line under it.
    pub sub: String,
    /// The tab shown.
    pub tab: VendorTab,
    /// How many rows wait on the To buy and To sell lists.
    pub buy_waiting: usize,
    pub sell_waiting: usize,
    /// What the player has (the shop's currency).
    pub purse: String,
    /// The player's pyreals beside another currency.
    pub purse_also: Option<String>,
    /// The rail's heading.
    pub rail_title: &'static str,
    /// The rail.
    pub rail: Vec<RailRow>,
    /// The rail's row picked.
    pub rail_on: u32,
    /// What the vendor buys, in words (the Sell tab).
    pub buys: Vec<String>,
    /// The list's heading, its count.
    pub list_title: String,
    pub list_count: String,
    /// The list's order, in words.
    pub sort: &'static str,
    /// The list.
    pub rows: Vec<VendorRow>,
    /// The chosen row.
    pub chosen: Option<VendorChosen>,
    /// The list's heading ("TO BUY").
    pub cart_title: &'static str,
    /// Its rows.
    pub cart: Vec<CartRow>,
    /// "Total" / "You receive", and the figure.
    pub total_words: &'static str,
    pub total: String,
    /// What the player has now and after.
    pub now: String,
    pub after: String,
    /// "After" goes below nothing.
    pub after_bad: bool,
    /// The list's button ("Buy all · 2,440p"), and whether it works.
    pub cart_button: String,
    pub can_cart: bool,
    /// The components the player is short of (Buy tab).
    pub restock: Option<Restock>,
}

/// What the window reads.
#[derive(Debug)]
pub struct ShopFacts<'a> {
    /// The shop.
    pub view: &'a VendorView,
    /// The player's items, the selection and requests in flight.
    pub items: &'a ItemsView<'a>,
    /// The restock amounts set in the Spellbook (none: no book yet).
    pub desired: Option<&'a BTreeMap<u32, u32>>,
    /// The player's pyreals.
    pub pyreals: i32,
}

/// The category a listing of `item_type` falls in: its caption and mask.
#[must_use]
pub fn category_of(item_type: u32) -> Option<(&'static str, u32)> {
    CATEGORY_FILTERS
        .iter()
        .find(|(_, mask)| item_type & mask != 0)
        .copied()
}

/// An amount of the shop's currency in words: "1,200p", or "8 tokens".
#[must_use]
pub fn money(view: &VendorView, amount: i32) -> String {
    let p = &view.profile;
    if p.alternate_currency_wcid != 0 {
        let name = p.alternate_currency_plural_name.trim();
        let name = if name.is_empty() { "tokens" } else { name };
        format!("{} {name}", group_thousands(amount))
    } else {
        format!("{}p", group_thousands(amount))
    }
}

/// What the vendor buys, in words: its kinds of goods, and the values and
/// magic it will not take.
#[must_use]
pub fn buys_words(view: &VendorView) -> Vec<String> {
    let p = &view.profile;
    let kinds: Vec<String> = CATEGORY_FILTERS
        .iter()
        .filter(|(_, mask)| p.merchandise_item_types & mask != 0)
        .map(|(words, _)| words.to_lowercase())
        .collect();
    let mut out = Vec::new();
    if kinds.is_empty() {
        out.push("Nothing.".to_owned());
        return out;
    }
    let mut first = kinds.join(", ");
    if let Some(c) = first.get_mut(0..1) {
        c.make_ascii_uppercase();
    }
    out.push(format!("{first}."));
    let mut limits = Vec::new();
    if p.merchandise_max_value != NO_LIMIT {
        limits.push(format!(
            "Up to {}p each.",
            group_thousands(i32::try_from(p.merchandise_max_value).unwrap_or(i32::MAX))
        ));
    }
    if p.merchandise_min_value != NO_LIMIT && p.merchandise_min_value > 0 {
        limits.push(format!(
            "From {}p each.",
            group_thousands(i32::try_from(p.merchandise_min_value).unwrap_or(i32::MAX))
        ));
    }
    if !p.deal_magical_items {
        limits.push("No magic items.".to_owned());
    }
    if !limits.is_empty() {
        out.push(limits.join(" "));
    }
    out
}

/// What the vendor pays for `quantity` of `o`.
#[must_use]
pub fn pays(view: &VendorView, o: &ClientObject, quantity: i32) -> i32 {
    buy_price(
        per_unit_value(o.value, Some(o.stack_size)),
        o.item_type.bits(),
        view.profile.buy_price,
        quantity,
    )
}

/// The player's items the Sell tab lists, each with the pack it is in
/// (the main pack first, then the side packs in order).
#[must_use]
pub fn carried(objects: &ClientObjectTable, player: u32) -> Vec<(u32, u32)> {
    let mut packs = vec![player];
    packs.extend(objects.get_contents(player).iter().copied().filter(|&g| {
        objects
            .get(g)
            .is_some_and(|o| is_pack(o) && o.currently_equipped_location == EquipMask::NONE)
    }));
    let mut out = Vec::new();
    for pack in packs {
        for id in loose_contents(objects, pack) {
            if objects
                .get(id)
                .is_some_and(|o| o.currently_equipped_location == EquipMask::NONE)
            {
                out.push((id, pack));
            }
        }
    }
    out
}

fn matches(search: &str, name: &str) -> bool {
    let s = search.trim();
    s.is_empty() || name.to_lowercase().contains(&s.to_lowercase())
}

/// The listing's stock in words.
fn stock_words(item: &VendorShopItem) -> String {
    if item.stack_size == UNLIMITED_STOCK || item.stack_size < 0 {
        "Unlimited".to_owned()
    } else {
        format!("{} left", item.stack_size)
    }
}

/// The most of a listing one purchase takes: what is left, and no more
/// than a stack (or 100 of a kind that does not stack).
fn most_of(item: &VendorShopItem) -> i32 {
    let stack = resolve_authored_stack_size(item.desc_stack_size, item.max_stack_size);
    let cap = if stack > 1 { stack } else { 100 };
    if item.stack_size == UNLIMITED_STOCK || item.stack_size < 0 {
        cap
    } else {
        item.stack_size.clamp(0, cap)
    }
}

/// The restock: what the player is short of that this shop stocks.
#[must_use]
pub fn restock(facts: &ShopFacts<'_>) -> Option<Restock> {
    let desired = facts.desired?;
    let view = facts.view;
    let desires = build_component_desires(
        desired,
        facts.items.objects,
        facts.items.player,
        |_| None,
        &view.items,
    );
    let staged: Vec<VendorStagingEntry> = view
        .lists
        .buying
        .iter()
        .map(|r| VendorStagingEntry {
            item_guid: r.item_guid,
            quantity: r.quantity,
        })
        .collect();
    let plan = plan_component_fill(
        &desires,
        ANY_CATEGORY,
        0,
        &view.items,
        &view.profile,
        &staged,
    );
    if plan.adds.is_empty() {
        return None;
    }
    let cost: i32 = plan
        .adds
        .iter()
        .map(|a| view.price(a.item_guid, a.quantity).unwrap_or(0))
        .fold(0, i32::saturating_add);
    Some(Restock {
        count: plan.adds.len(),
        cost: money(view, cost),
        adds: plan
            .adds
            .iter()
            .map(|a| (a.item_guid, a.quantity))
            .collect(),
    })
}

/// How many of the listing `wcid` the player carries, and the restock
/// amount set for it.
fn component_need(facts: &ShopFacts<'_>, wcid: u32) -> Option<(i32, i32)> {
    let want = *facts.desired?.get(&wcid)?;
    if want == 0 {
        return None;
    }
    let objects = facts.items.objects;
    let have = objects
        .objects()
        .filter(|o| {
            o.weenie_class_id == wcid
                && objects.is_owned_by_object(o.object_id(), facts.items.player)
        })
        .map(|o| o.stack_size.max(1))
        .fold(0i32, i32::saturating_add);
    Some((have, i32::try_from(want).unwrap_or(i32::MAX)))
}

/// The Vendor window's view of `facts` as the player picked `choices`;
/// `icon` makes an item's icon.
#[allow(clippy::too_many_lines)]
pub fn screen(
    facts: &ShopFacts<'_>,
    choices: &VendorChoices,
    icon: &mut dyn FnMut(Iconed) -> Option<TextureRef>,
    names: &dyn Fn(&ClientObject) -> String,
    mark: &dyn Fn(u32) -> Option<(String, ac_ui::draw::Color)>,
) -> VendorScreen {
    let view = facts.view;
    let objects = facts.items.objects;
    let player = facts.items.player;
    let alternate = view.profile.alternate_currency_wcid != 0;
    let have = view.lists.money;
    let name = view
        .name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "Vendor".to_owned());
    let sub = if alternate {
        format!(
            "Takes {}, not pyreals",
            view.profile.alternate_currency_plural_name.trim()
        )
    } else {
        "Shop".to_owned()
    };
    let mut out = VendorScreen {
        name,
        sub,
        tab: choices.tab,
        buy_waiting: view.lists.buying.len(),
        sell_waiting: view.lists.selling.len(),
        purse: money(view, have),
        purse_also: alternate.then(|| format!("{}p", group_thousands(facts.pyreals))),
        buys: buys_words(view),
        sort: choices.sort.words(choices.tab),
        ..VendorScreen::default()
    };
    match choices.tab {
        VendorTab::Buy => {
            // The categories this shop stocks.
            let mut counts: Vec<(&'static str, u32, usize)> = Vec::new();
            for item in &view.items {
                if let Some((words, mask)) = category_of(item.item_type.unwrap_or(0)) {
                    match counts.iter_mut().find(|(_, m, _)| *m == mask) {
                        Some(c) => c.2 += 1,
                        None => counts.push((words, mask, 1)),
                    }
                }
            }
            counts.sort_by_key(|c| c.0);
            out.rail_title = "WHAT THEY SELL";
            out.rail.push(RailRow {
                words: "All goods".to_owned(),
                key: 0,
                count: view.items.len(),
            });
            out.rail.extend(counts.iter().map(|(w, m, n)| RailRow {
                words: (*w).to_owned(),
                key: *m,
                count: *n,
            }));
            let category = if counts.iter().any(|c| c.1 == choices.category) {
                choices.category
            } else {
                0
            };
            out.rail_on = category;
            let on_list = |guid: u32| {
                view.lists
                    .buying
                    .iter()
                    .find(|r| r.item_guid == guid)
                    .map(|r| r.quantity)
            };
            let mut goods: Vec<&VendorShopItem> = view
                .items
                .iter()
                .filter(|i| category == 0 || i.item_type.unwrap_or(0) & category != 0)
                .filter(|i| matches(&choices.search, i.name.as_deref().unwrap_or("")))
                .collect();
            let price_of = |i: &VendorShopItem| listing_price(i, &view.profile, 1);
            match choices.sort {
                VendorSort::Group => goods.sort_by(|a, b| {
                    let ca = category_of(a.item_type.unwrap_or(0)).map(|c| c.0);
                    let cb = category_of(b.item_type.unwrap_or(0)).map(|c| c.0);
                    ca.cmp(&cb).then_with(|| a.name.cmp(&b.name))
                }),
                VendorSort::Name => goods.sort_by(|a, b| a.name.cmp(&b.name)),
                VendorSort::Price => goods.sort_by_key(|i| std::cmp::Reverse(price_of(i))),
            }
            for item in &goods {
                let category = category_of(item.item_type.unwrap_or(0)).map_or("Goods", |c| c.0);
                let (sub, sub_tone) = match component_need(facts, item.weenie_class_id) {
                    Some((carry, to)) if carry < to => (
                        format!(
                            "You carry {carry} \u{b7} restock to {to} \u{b7} need {}",
                            to - carry
                        ),
                        if carry == 0 { Tone::Bad } else { Tone::Warn },
                    ),
                    Some((carry, to)) => (
                        format!("You carry {carry} \u{b7} restock to {to}"),
                        Tone::Quiet,
                    ),
                    None => (
                        format!("{category} \u{b7} {}", stock_words(item)),
                        Tone::Quiet,
                    ),
                };
                out.rows.push(VendorRow {
                    id: item.item_guid,
                    icon: icon(Iconed::Listing(item)),
                    name: item.name.clone().unwrap_or_default(),
                    sub,
                    sub_tone,
                    on_list: on_list(item.item_guid).map(|n| format!("On list \u{d7}{n}")),
                    price: money(view, price_of(item)),
                    price_sub: "each".to_owned(),
                    refused: false,
                    stack: 1,
                    mark: mark(item.item_guid),
                });
            }
            out.list_title = out
                .rail
                .iter()
                .find(|r| r.key == category)
                .map_or_else(|| "All goods".to_owned(), |r| r.words.clone());
            out.list_count = count_words(out.rows.len());
            out.restock = restock(facts).filter(|_| {
                category == 0 || category == ac_ui::panels::vendor::SPELL_COMPONENTS_MASK
            });
            // The chosen listing.
            let chosen = choices
                .chosen
                .and_then(|g| goods.iter().copied().find(|i| i.item_guid == g))
                .or_else(|| goods.first().copied());
            if let Some(item) = chosen {
                let most = most_of(item).max(1);
                let need = component_need(facts, item.weenie_class_id)
                    .map(|(carry, to)| (to - carry).max(0))
                    .filter(|&n| n > 0);
                let quantity = choices.quantity.unwrap_or(1).clamp(1, most);
                let total = listing_price(item, &view.profile, quantity);
                let listing = std::slice::from_ref(item);
                let room = has_room_in(&[(quantity, item.item_guid)], listing, objects, player);
                let afford = total <= have;
                let (note, note_tone) = if !afford {
                    (
                        format!("You have {}: not enough.", money(view, have)),
                        Tone::Bad,
                    )
                } else if !room {
                    (
                        "There is no room for it in your main pack.".to_owned(),
                        Tone::Bad,
                    )
                } else if need.is_some() {
                    let to = component_need(facts, item.weenie_class_id).map_or(0, |c| c.1);
                    (
                        format!("Brings you to your restock amount of {to}."),
                        Tone::Quiet,
                    )
                } else {
                    (
                        format!("Fits in your main pack. You have {}.", money(view, have)),
                        Tone::Quiet,
                    )
                };
                let category = category_of(item.item_type.unwrap_or(0)).map_or("Goods", |c| c.0);
                out.chosen = Some(VendorChosen {
                    id: item.item_guid,
                    icon: icon(Iconed::Listing(item)),
                    name: item.name.clone().unwrap_or_default(),
                    sub: format!(
                        "{category} \u{b7} {} each \u{b7} {}",
                        money(view, listing_price(item, &view.profile, 1)),
                        stock_words(item).to_lowercase()
                    ),
                    quantity,
                    most: need.map_or(most, |n| n.min(most)),
                    most_words: if need.is_some() { "To restock" } else { "Max" },
                    total: money(view, total),
                    total_tone: Tone::Quiet,
                    note,
                    note_tone,
                    primary: "Buy now",
                    can_primary: afford && room && facts.items.can_make_request,
                    can_add: true,
                });
            }
            // The To buy list.
            out.cart_title = "TO BUY";
            out.cart = view
                .lists
                .buying
                .iter()
                .map(|r| CartRow {
                    id: r.item_guid,
                    icon: view
                        .items
                        .iter()
                        .find(|i| i.item_guid == r.item_guid)
                        .and_then(|i| icon(Iconed::Listing(i))),
                    name: r.name.clone(),
                    quantity: r.quantity,
                    price: money(view, r.price),
                })
                .collect();
            let total: i32 = view
                .lists
                .buying
                .iter()
                .map(|r| r.price)
                .fold(0, i32::saturating_add);
            out.total_words = "Total";
            out.total = money(view, total);
            out.now = format!("You have {}", money(view, have));
            out.after = format!("After: {}", money(view, have.saturating_sub(total)));
            out.after_bad = total > have;
            out.cart_button = if total > 0 {
                format!("Buy all \u{b7} {}", money(view, total))
            } else {
                "Buy all".to_owned()
            };
            out.can_cart = !out.cart.is_empty() && total <= have && facts.items.can_make_request;
        }
        VendorTab::Sell => {
            let mine = carried(objects, player);
            let mut packs: Vec<u32> = Vec::new();
            for (_, pack) in &mine {
                if !packs.contains(pack) {
                    packs.push(*pack);
                }
            }
            let pack_name = |pack: u32| {
                if pack == player {
                    "Main pack".to_owned()
                } else {
                    objects
                        .get(pack)
                        .map_or_else(|| "Pack".to_owned(), |o| o.appropriate_name().into_owned())
                }
            };
            out.rail_title = "YOUR PACKS";
            out.rail.push(RailRow {
                words: "All packs".to_owned(),
                key: 0,
                count: mine.len(),
            });
            for pack in &packs {
                out.rail.push(RailRow {
                    words: pack_name(*pack),
                    key: *pack,
                    count: mine.iter().filter(|(_, p)| p == pack).count(),
                });
            }
            let pack = if packs.contains(&choices.pack) {
                choices.pack
            } else {
                0
            };
            out.rail_on = pack;
            let selling = |guid: u32| {
                view.lists
                    .selling
                    .iter()
                    .find(|r| r.item_guid == guid)
                    .map(|r| r.quantity)
            };
            let mut rows: Vec<(VendorRow, u32, i32)> = Vec::new();
            for (id, in_pack) in &mine {
                if pack != 0 && *in_pack != pack {
                    continue;
                }
                let Some(o) = objects.get(*id) else {
                    continue;
                };
                let name = names(o);
                if !matches(&choices.search, &name) {
                    continue;
                }
                let refusal = sell_check(facts.items, &view.profile, *id).err();
                if refusal.is_some() && choices.only_taken {
                    continue;
                }
                let stack = o.stack_size.max(1);
                let each = pays(view, o, 1);
                let kind = category_of(o.item_type.bits()).map_or("Goods", |c| c.0);
                let sub = match refusal {
                    Some(r) => r.message().to_owned(),
                    None if stack > 1 => {
                        format!(
                            "{} \u{b7} {kind} \u{b7} {stack} in stack",
                            pack_name(*in_pack)
                        )
                    }
                    None => format!("{} \u{b7} {kind}", pack_name(*in_pack)),
                };
                rows.push((
                    VendorRow {
                        id: *id,
                        icon: icon(Iconed::Object(o)),
                        name,
                        sub,
                        sub_tone: if refusal.is_some() {
                            Tone::Bad
                        } else {
                            Tone::Quiet
                        },
                        on_list: selling(*id).map(|n| format!("Selling \u{d7}{n}")),
                        price: if refusal.is_some() {
                            "\u{2014}".to_owned()
                        } else {
                            money(view, each)
                        },
                        price_sub: if refusal.is_some() {
                            String::new()
                        } else {
                            "they pay each".to_owned()
                        },
                        refused: refusal.is_some(),
                        stack,
                        mark: mark(*id),
                    },
                    *in_pack,
                    if refusal.is_some() {
                        -1
                    } else {
                        pays(view, o, stack)
                    },
                ));
            }
            match choices.sort {
                VendorSort::Group => {}
                VendorSort::Name => rows.sort_by(|a, b| a.0.name.cmp(&b.0.name)),
                VendorSort::Price => rows.sort_by_key(|r| std::cmp::Reverse(r.2)),
            }
            let taken = rows.iter().filter(|r| !r.0.refused).count();
            out.list_count = format!(
                "{taken} they will buy \u{b7} {} they will not",
                rows.len() - taken
            );
            out.rows = rows.into_iter().map(|r| r.0).collect();
            out.list_title = if pack == 0 {
                "Everything you carry".to_owned()
            } else {
                pack_name(pack)
            };
            // The chosen item.
            let chosen = choices
                .chosen
                .filter(|g| out.rows.iter().any(|r| r.id == *g))
                .or_else(|| out.rows.first().map(|r| r.id));
            if let Some(o) = chosen.and_then(|g| objects.get(g)) {
                let id = o.object_id();
                let stack = o.stack_size.max(1);
                let refusal = sell_check(facts.items, &view.profile, id).err();
                let quantity = choices.quantity.unwrap_or(stack).clamp(1, stack);
                let kind = category_of(o.item_type.bits()).map_or("Goods", |c| c.0);
                let (note, note_tone) = match refusal {
                    Some(_) => (format!("{} will not take it.", out.name), Tone::Bad),
                    None if quantity < stack => (
                        "Part of a stack is split off before it is sold.".to_owned(),
                        Tone::Quiet,
                    ),
                    None => (String::new(), Tone::Quiet),
                };
                out.chosen = Some(VendorChosen {
                    id,
                    icon: icon(Iconed::Object(o)),
                    name: names(o),
                    sub: match refusal {
                        Some(r) => r.message().to_owned(),
                        None if stack > 1 => format!("{kind} \u{b7} {stack} in stack"),
                        None => kind.to_owned(),
                    },
                    quantity,
                    most: stack,
                    most_words: "All",
                    total: if refusal.is_some() {
                        "\u{2014}".to_owned()
                    } else {
                        money(view, pays(view, o, quantity))
                    },
                    total_tone: if refusal.is_some() {
                        Tone::Quiet
                    } else {
                        Tone::Good
                    },
                    note,
                    note_tone,
                    primary: "Sell now",
                    can_primary: refusal.is_none() && facts.items.can_make_request,
                    can_add: refusal.is_none(),
                });
            }
            // The To sell list.
            out.cart_title = "TO SELL";
            out.cart = view
                .lists
                .selling
                .iter()
                .map(|r| CartRow {
                    id: r.item_guid,
                    icon: objects
                        .get(r.item_guid)
                        .and_then(|o| icon(Iconed::Object(o))),
                    name: r.name.clone(),
                    quantity: r.quantity,
                    price: money(view, r.price),
                })
                .collect();
            let total: i32 = view
                .lists
                .selling
                .iter()
                .map(|r| r.price)
                .fold(0, i32::saturating_add);
            out.total_words = "You receive";
            out.total = money(view, total);
            // What the player receives is pyreals whatever the shop sells
            // for.
            let pyreals = if alternate { facts.pyreals } else { have };
            out.now = format!("You have {}p", group_thousands(pyreals));
            out.after = format!("After: {}p", group_thousands(pyreals.saturating_add(total)));
            out.cart_button = if total > 0 {
                format!("Sell all \u{b7} {}", money(view, total))
            } else {
                "Sell all".to_owned()
            };
            out.can_cart = !out.cart.is_empty() && facts.items.can_make_request;
        }
    }
    out
}

/// What an icon is made from.
#[derive(Debug, Clone, Copy)]
pub enum Iconed<'a> {
    /// A shop listing.
    Listing(&'a VendorShopItem),
    /// One of the player's items.
    Object(&'a ClientObject),
}

fn count_words(n: usize) -> String {
    if n == 1 {
        "1 item".to_owned()
    } else {
        format!("{n} items")
    }
}

/// What the player is told beside `why` the vendor will not take `o`: the
/// vendor's limit against the item's worth, when a limit is the reason.
#[must_use]
pub fn refusal_limit(
    view: &VendorView,
    o: &ClientObject,
    vendor: &str,
    why: VendorSellRejection,
) -> Option<String> {
    let each = group_thousands(per_unit_value(o.value, Some(o.stack_size)));
    let p = &view.profile;
    let limit = |v: u32| group_thousands(i32::try_from(v).unwrap_or(i32::MAX));
    match why {
        VendorSellRejection::TooValuable => Some(format!(
            "{vendor} buys up to {}p. It is worth {each}p.",
            limit(p.merchandise_max_value)
        )),
        VendorSellRejection::TooCheap => Some(format!(
            "{vendor} buys from {}p. It is worth {each}p.",
            limit(p.merchandise_min_value)
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_core::items::{ItemType, VendorShopProfile};
    use ac_runtime::inventory::VendorViewReason;
    use ac_runtime::inventory::shop::{StagedRow, VendorListsView};

    const PLAYER: u32 = 0x5000_0001;

    fn listing(guid: u32, name: &str, item_type: ItemType, value: i32) -> VendorShopItem {
        VendorShopItem {
            item_guid: guid,
            stack_size: UNLIMITED_STOCK,
            name: Some(name.to_owned()),
            item_type: Some(item_type.bits()),
            value: Some(value),
            ..VendorShopItem::default()
        }
    }

    fn shop() -> VendorView {
        VendorView {
            vendor_guid: 0x7000_0001,
            name: Some("Ilsa".to_owned()),
            profile: VendorShopProfile {
                merchandise_item_types: ItemType::GEM.bits() | ItemType::FOOD.bits(),
                merchandise_min_value: NO_LIMIT,
                merchandise_max_value: 25_000,
                deal_magical_items: false,
                buy_price: 0.5,
                sell_price: 1.0,
                ..VendorShopProfile::default()
            },
            items: vec![
                listing(0x8000_0001, "Apple", ItemType::FOOD, 4),
                listing(0x8000_0002, "Amethyst", ItemType::GEM, 250),
                listing(0x8000_0003, "Bread", ItemType::FOOD, 6),
            ],
            lists: VendorListsView {
                buying: vec![StagedRow {
                    item_guid: 0x8000_0001,
                    name: "Apple".to_owned(),
                    quantity: 10,
                    price: 40,
                }],
                money: 100,
                ..VendorListsView::default()
            },
            reason: VendorViewReason::Opened,
        }
    }

    fn with_view<R>(view: &VendorView, f: impl FnOnce(&ShopFacts<'_>) -> R) -> R {
        let mut objects = ClientObjectTable::new();
        let mut player = ClientObject::new(PLAYER);
        player.name = "Me".to_owned();
        player.items_capacity = 102;
        let mut ev = Vec::new();
        objects.add_or_update(player, &mut ev);
        let mut items = ItemsView::new(&objects, PLAYER);
        items.can_make_request = true;
        let facts = ShopFacts {
            view,
            items: &items,
            desired: None,
            pyreals: 100,
        };
        f(&facts)
    }

    fn no_icon(_: Iconed<'_>) -> Option<TextureRef> {
        None
    }

    #[test]
    fn the_limit_is_told_only_when_it_is_the_reason() {
        let view = shop();
        let mut gem = ClientObject::new(0x5000_0101);
        gem.value = 30_000;
        gem.stack_size = 1;
        assert_eq!(
            refusal_limit(&view, &gem, "Ilsa", VendorSellRejection::TooValuable).as_deref(),
            Some("Ilsa buys up to 25,000p. It is worth 30,000p.")
        );
        assert_eq!(
            refusal_limit(&view, &gem, "Ilsa", VendorSellRejection::WrongType),
            None
        );
    }

    #[test]
    fn the_buy_tab_lists_the_shops_categories_and_goods() {
        let view = shop();
        let s = with_view(&view, |f| {
            screen(
                f,
                &VendorChoices::default(),
                &mut no_icon,
                &|o| o.name.clone(),
                &|_| None,
            )
        });
        let rail: Vec<(&str, usize)> = s.rail.iter().map(|r| (r.words.as_str(), r.count)).collect();
        assert_eq!(rail, [("All goods", 3), ("Food", 2), ("Gems", 1)]);
        let names: Vec<&str> = s.rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            ["Apple", "Bread", "Amethyst"],
            "by category, then name"
        );
        assert_eq!(s.rows[0].on_list.as_deref(), Some("On list \u{d7}10"));
        assert_eq!(s.buy_waiting, 1);
        assert_eq!(s.total, "40p");
        assert_eq!(s.after, "After: 60p");
    }

    #[test]
    fn a_category_narrows_the_goods_and_a_dear_quantity_is_refused() {
        let view = shop();
        let choices = VendorChoices {
            category: ItemType::GEM.bits(),
            quantity: Some(1),
            ..VendorChoices::default()
        };
        let s = with_view(&view, |f| {
            screen(f, &choices, &mut no_icon, &|o| o.name.clone(), &|_| None)
        });
        assert_eq!(s.rows.len(), 1);
        let c = s.chosen.expect("the amethyst");
        assert_eq!(c.total, "250p");
        assert!(!c.can_primary, "100p does not pay for 250p");
        assert_eq!(c.note_tone, Tone::Bad);
    }

    #[test]
    fn what_the_vendor_buys_is_said_in_words() {
        let words = buys_words(&shop());
        assert_eq!(words[0], "Food, gems.");
        assert_eq!(words[1], "Up to 25,000p each. No magic items.");
    }

    #[test]
    fn another_currency_names_itself() {
        let mut view = shop();
        view.profile.alternate_currency_wcid = 42;
        view.profile.alternate_currency_plural_name = "Pathwarden Tokens".to_owned();
        assert_eq!(money(&view, 18), "18 Pathwarden Tokens");
    }
}
