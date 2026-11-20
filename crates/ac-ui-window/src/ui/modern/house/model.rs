//! What the House window and the house sale window show, worked out from
//! the session's house, the slumlord's profile and what the player carries.

use ac_core::items::ClientObjectTable;
use ac_runtime::gameplay::{HouseAccessView, HouseView};
use ac_runtime::inventory::{HousePurchaseTab, HousePurchaseView};
use ac_ui::modern::numbers::group_thousands;
use ac_ui::radar::RadarCoordinates;

/// A house type's name.
#[must_use]
pub const fn kind_name(house_type: u32) -> &'static str {
    match house_type {
        1 => "Cottage",
        2 => "Villa",
        3 => "Mansion",
        4 => "Apartment",
        _ => "House",
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Days since 1970-01-01 to a (year, month, day).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// A moment as the windows word it ("2 December 2026, 19:41"), local time
/// `offset` seconds from UTC.
#[must_use]
pub fn long_date(unix_s: i64, offset: i32) -> String {
    let local = unix_s + i64::from(offset);
    let (y, m, d) = civil_from_days(local.div_euclid(86_400));
    let s = local.rem_euclid(86_400);
    let month = MONTHS[usize::try_from(m - 1).unwrap_or(0).min(11)];
    format!("{d} {month} {y}, {:02}:{:02}", s / 3600, s / 60 % 60)
}

/// A cell's coordinates ("42.1N, 33.9E").
#[must_use]
pub fn coordinates(cell: u32) -> Option<String> {
    RadarCoordinates::from_cell(cell).map(|c| format!("{}, {}", c.y_text(), c.x_text()))
}

/// How a maintenance item stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// All of it is paid.
    Paid,
    /// Some is still owed, and the period is still paid.
    Owed,
    /// Some is still owed and the house is at risk.
    Due,
}

/// A maintenance item's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RentRow {
    /// "50,000 Pyreals".
    pub name: String,
    /// "Paid" or "3 of 5".
    pub text: String,
    /// How much is paid, 0 to 1 in thousandths.
    pub share: u32,
    /// How it stands.
    pub standing: Standing,
}

/// The player's house.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedHouse {
    /// "Cottage".
    pub kind: &'static str,
    /// Its coordinates (none: an apartment).
    pub place: Option<String>,
    /// Its cell (0: an apartment).
    pub cell: u32,
    /// Bought, this period ends, next due (the second red when unpaid).
    pub rows: Vec<(&'static str, String, bool)>,
    /// Each maintenance item.
    pub rent: Vec<RentRow>,
    /// "PAID", "NOT PAID" or "FREE".
    pub period: &'static str,
    /// The note under the items, and whether it warns.
    pub note: String,
    pub warns: bool,
}

/// The House window.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HouseScreen {
    /// The house (none: the player owns none).
    pub owned: Option<OwnedHouse>,
    /// When another house may be bought, worded.
    pub buy_another: String,
    /// Who may come in, as the server last said.
    pub access: Option<HouseAccessView>,
    /// No word from the server about the house yet.
    pub waiting: bool,
}

/// The House window for `view`, local time `offset` seconds from UTC.
#[must_use]
pub fn house_screen(view: &HouseView, offset: i32) -> HouseScreen {
    let date = |s: i64| long_date(s, offset);
    let owned = view.details.as_ref().filter(|_| view.owns_house).map(|d| {
        let due = !d.paid && !d.maintenance_free;
        let days = if d.house_type == 4 { 90 } else { 30 };
        let rent = d
            .rent
            .iter()
            .map(|(name, paid, num)| {
                let full = paid >= num;
                RentRow {
                    name: if *num > 1 {
                        format!("{} {name}", group_thousands(i64::from(*num)))
                    } else {
                        name.clone()
                    },
                    text: if full {
                        "Paid".to_owned()
                    } else {
                        format!("{paid} of {num}")
                    },
                    share: if *num == 0 {
                        1000
                    } else {
                        (paid.min(num) * 1000) / num
                    },
                    standing: if full {
                        Standing::Paid
                    } else if due {
                        Standing::Due
                    } else {
                        Standing::Owed
                    },
                }
            })
            .collect();
        let (note, warns) = if d.maintenance_free {
            ("This house needs no maintenance.".to_owned(), false)
        } else if d.paid {
            (
                "Paid for this period. The next period cannot be paid early.".to_owned(),
                false,
            )
        } else {
            (
                format!(
                    "Maintenance for the last {days}-day period is not paid. Pay it by {}, or you lose the house and everything in it.",
                    date(d.period_ends_unix_s)
                ),
                true,
            )
        };
        OwnedHouse {
            kind: kind_name(d.house_type),
            place: (d.cell != 0).then(|| coordinates(d.cell)).flatten(),
            cell: d.cell,
            rows: vec![
                ("Bought", date(d.bought_unix_s), false),
                ("This period ends", date(d.period_ends_unix_s), due),
                ("Next due", date(d.next_due_unix_s), false),
            ],
            rent,
            period: if d.maintenance_free {
                "FREE"
            } else if d.paid {
                "PAID"
            } else {
                "NOT PAID"
            },
            note,
            warns,
        }
    });
    let buy_another = if view.buy_another_unix_s > 0 {
        format!(
            "You may buy another landscape house on {}. Apartments are not limited.",
            date(view.buy_another_unix_s)
        )
    } else if owned.is_some() {
        "You may buy another house as soon as you abandon this one.".to_owned()
    } else {
        "You may buy a house now.".to_owned()
    };
    HouseScreen {
        owned,
        buy_another,
        access: view.access.clone(),
        waiting: !view.has_received_notice,
    }
}

/// A rule of the house sale window: what, its value, and whether the
/// player meets it (none: nothing to meet).
pub type Rule = (String, String, Option<bool>);

/// One price line of the house sale window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceRow {
    /// Its line in the profile.
    pub line: usize,
    /// What it takes.
    pub wcid: u32,
    /// "50,000 Pyreals".
    pub name: String,
    /// "you carry 61,200".
    pub carry: String,
    /// The player carries some.
    pub carries: bool,
    /// How much is paid (the server's, for maintenance) and on the row.
    pub paid: i32,
    pub staged: i32,
    /// How much it takes.
    pub num: i32,
}

impl PriceRow {
    /// Nothing more is owed on it.
    #[must_use]
    pub fn full(&self) -> bool {
        self.paid + self.staged >= self.num
    }
}

/// The house sale window.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SaleScreen {
    /// "Cottage".
    pub kind: &'static str,
    /// The owner pill's words, and whether it is the player's own.
    pub owner: String,
    pub yours: bool,
    /// Nobody owns it.
    pub for_sale: bool,
    /// The rules shown above the prices.
    pub rules: Vec<Rule>,
    /// The price lines.
    pub rows: Vec<PriceRow>,
    /// What is on the rows (id, name).
    pub staged: Vec<(u32, String)>,
    /// The line under the rows.
    pub hint: String,
    /// The foot's words and the button's.
    pub foot: String,
    pub action: &'static str,
    /// The button may send.
    pub can: bool,
}

/// What the house sale window is worked out from.
#[derive(Debug)]
pub struct SaleInput<'a> {
    /// The slumlord window.
    pub view: &'a HousePurchaseView,
    /// The tab shown.
    pub tab: HousePurchaseTab,
    /// The objects known, and the player.
    pub objects: &'a ClientObjectTable,
    pub player: u32,
    /// The player's level and allegiance rank.
    pub level: i32,
    pub rank: i32,
    /// When another landscape house may be bought (0: now), and the local
    /// offset for words.
    pub buy_another_unix_s: i64,
    pub offset: i32,
}

/// What the player carries of weenie class `wcid`, counting stacks.
fn carried(objects: &ClientObjectTable, player: u32, wcid: u32) -> (i32, Vec<u32>) {
    let mut count = 0;
    let mut ids = Vec::new();
    for o in objects.objects() {
        if o.weenie_class_id != wcid || o.object_id() == player {
            continue;
        }
        let holder = o.container_id;
        let carried = holder == player
            || objects
                .get(holder)
                .is_some_and(|c| c.container_id == player);
        if carried && o.wielder_id == 0 {
            count += o.stack_size.max(1);
            ids.push(o.object_id());
        }
    }
    (count, ids)
}

/// The items the player carries that pay line `wcid`.
#[must_use]
pub fn payers(objects: &ClientObjectTable, player: u32, wcid: u32) -> Vec<u32> {
    carried(objects, player, wcid).1
}

fn range(min: i32, max: i32) -> Option<String> {
    match (min >= 0, max >= 0) {
        (true, true) => Some(format!("{min} to {max}")),
        (true, false) => Some(format!("{min} or more")),
        (false, true) => Some(format!("{max} or less")),
        (false, false) => None,
    }
}

/// The house sale window for `input` (none: no slumlord window).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn sale_screen(input: &SaleInput<'_>) -> Option<SaleScreen> {
    let profile = input.view.profile.as_ref()?;
    let tab = input.tab;
    let yours = profile.owner_guid != 0 && profile.owner_guid == input.player;
    let for_sale = profile.owner_guid == 0;
    let owner = if for_sale {
        "For sale".to_owned()
    } else if yours {
        "Yours".to_owned()
    } else {
        format!("Owned by {}", profile.owner_name)
    };
    let mut rules: Vec<Rule> = Vec::new();
    match tab {
        HousePurchaseTab::Buy => {
            if let Some(r) = range(profile.min_level, profile.max_level) {
                let ok = (profile.min_level < 0 || input.level >= profile.min_level)
                    && (profile.max_level < 0 || input.level <= profile.max_level);
                rules.push(("Level".to_owned(), r, Some(ok)));
            }
            if let Some(r) = range(profile.min_allegiance_rank, profile.max_allegiance_rank) {
                let ok = (profile.min_allegiance_rank < 0
                    || input.rank >= profile.min_allegiance_rank)
                    && (profile.max_allegiance_rank < 0
                        || input.rank <= profile.max_allegiance_rank);
                rules.push(("Allegiance rank".to_owned(), r, Some(ok)));
            }
            if profile.house_type != 4 {
                let now = input.buy_another_unix_s == 0;
                rules.push((
                    "Another landscape house".to_owned(),
                    if now {
                        "You may buy one now".to_owned()
                    } else {
                        format!("From {}", long_date(input.buy_another_unix_s, input.offset))
                    },
                    Some(now),
                ));
            }
            if !for_sale {
                rules.push(("Owner".to_owned(), profile.owner_name.clone(), Some(false)));
            }
        }
        HousePurchaseTab::Rent => {
            rules.push((
                "Owner".to_owned(),
                if for_sale {
                    "Nobody".to_owned()
                } else if yours {
                    "You".to_owned()
                } else {
                    profile.owner_name.clone()
                },
                None,
            ));
            rules.push((
                "Maintenance".to_owned(),
                if profile.maintenance_free {
                    "Free".to_owned()
                } else {
                    "Every 30 days".to_owned()
                },
                None,
            ));
        }
    }
    let lines = match tab {
        HousePurchaseTab::Buy => &profile.buy,
        HousePurchaseTab::Rent => &profile.rent,
    };
    let rows: Vec<PriceRow> = lines
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (count, _) = carried(input.objects, input.player, p.weenie_id);
            PriceRow {
                line: i,
                wcid: p.weenie_id,
                name: if p.num > 1 {
                    format!(
                        "{} {}",
                        group_thousands(i64::from(p.num)),
                        ac_runtime::gameplay::house::payment_name(p)
                    )
                } else {
                    ac_runtime::gameplay::house::payment_name(p)
                },
                carry: format!("you carry {}", group_thousands(i64::from(count))),
                carries: count > 0,
                paid: if tab == HousePurchaseTab::Rent {
                    p.paid
                } else {
                    0
                },
                staged: input.view.staged_amount(tab, i),
                num: p.num,
            }
        })
        .collect();
    let staged = input
        .view
        .staged(tab)
        .iter()
        .map(|id| {
            let name = input.objects.get(*id).map_or_else(String::new, |o| {
                if o.stack_size > 1 {
                    format!("{} {}", group_thousands(i64::from(o.stack_size)), o.name)
                } else {
                    o.name.clone()
                }
            });
            (*id, name)
        })
        .collect();
    let (hint, foot, action) = match tab {
        HousePurchaseTab::Buy => (
            "Drag items onto a row or press Add; click an item on the rows to take it off.",
            if for_sale {
                "Buy is ready once every row is covered. It asks first."
            } else {
                "Someone owns this house; it is not for sale."
            },
            "Buy",
        ),
        HousePurchaseTab::Rent => (
            "Part payments are kept. Drag items onto a row or press Add.",
            if profile.maintenance_free {
                "This house needs no maintenance."
            } else if for_sale {
                "Nobody owns this house, so no maintenance is due."
            } else if yours {
                "Pays what is on the rows towards this period."
            } else {
                "This is someone else's house. Paying asks first."
            },
            "Pay maintenance",
        ),
    };
    Some(SaleScreen {
        kind: kind_name(profile.house_type),
        owner,
        yours,
        for_sale,
        rules,
        rows,
        staged,
        hint: hint.to_owned(),
        foot: foot.to_owned(),
        action,
        can: input.view.can_submit(tab),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_runtime::gameplay::HouseDetails;

    #[test]
    fn dates_read_day_month_year_and_time() {
        // 2 December 2026, 19:41 UTC.
        assert_eq!(long_date(1_796_240_460, 0), "2 December 2026, 19:41");
        assert_eq!(long_date(1_796_240_460, 3600), "2 December 2026, 20:41");
    }

    #[test]
    fn an_unpaid_house_warns_and_its_items_read_what_is_owed() {
        let view = HouseView {
            has_received_notice: true,
            owns_house: true,
            details: Some(HouseDetails {
                house_type: 1,
                cell: 0xA9B4_0031,
                bought_unix_s: 1_788_000_000,
                period_ends_unix_s: 1_796_240_460,
                next_due_unix_s: 1_796_240_460,
                maintenance_free: false,
                paid: false,
                rent: vec![
                    ("Pyreals".to_owned(), 50_000, 50_000),
                    ("Mana Scarabs".to_owned(), 3, 5),
                ],
            }),
            ..HouseView::default()
        };
        let s = house_screen(&view, 0);
        let h = s.owned.expect("owned");
        assert_eq!(h.kind, "Cottage");
        assert!(h.place.is_some());
        assert_eq!(h.period, "NOT PAID");
        assert!(h.warns);
        assert!(h.note.contains("2 December 2026, 19:41"));
        assert_eq!(h.rent[0].name, "50,000 Pyreals");
        assert_eq!(h.rent[0].text, "Paid");
        assert_eq!(h.rent[1].text, "3 of 5");
        assert_eq!(h.rent[1].standing, Standing::Due);
        assert!(h.rows[1].2, "the period's end is red");
        assert_eq!(
            s.buy_another,
            "You may buy another house as soon as you abandon this one."
        );
    }
}
