//! What a vendor window decides whatever it looks like: whether the vendor
//! takes an item, how much of a listing a bought or cleared row gives up,
//! and selling part of a stack (split it off, then stage the new stack once
//! it is in the pack).

use ac_core::items::vendor::{VendorSellCandidate, evaluate_sell, per_unit_value};
use ac_core::items::{ClientObject, VendorSellRejection, VendorShopItem, VendorShopProfile};
use ac_runtime::inventory::{ItemCommand, ObjectRef};
use ac_runtime::session::SessionCommand;

use crate::panels::items::ItemsView;

/// A split before selling could not be asked for.
pub const CANNOT_SPLIT_TO_SELL_MESSAGE: &str = "Cannot split the stack to sell it";

/// Whether the vendor (`profile`) takes the player's item `guid`.
///
/// # Errors
///
/// The vendor's reason when it does not.
pub fn sell_check(
    items: &ItemsView<'_>,
    profile: &VendorShopProfile,
    guid: u32,
) -> Result<(), VendorSellRejection> {
    let Some(item) = items.get(guid) else {
        return Err(VendorSellRejection::WrongType);
    };
    let player = items.player;
    evaluate_sell(
        &VendorSellCandidate {
            owned_by_player: items.objects.is_owned_by_object(guid, player),
            contained_item_count: i32::try_from(items.objects.get_contained_items(guid).len())
                .unwrap_or(i32::MAX),
            item_type: item.item_type.bits(),
            per_unit_value: per_unit_value(item.value, Some(item.stack_size)),
            public_weenie_bitfield: item.public_weenie_bitfield.map_or(0, |f| f.bits()),
        },
        profile,
    )
}

/// How much of a listing the buy list gives up when its row is bought or
/// cleared: the whole row for a stacking kind, else one.
#[must_use]
pub fn removal_amount(item: &VendorShopItem) -> i32 {
    if item.max_stack_size.unwrap_or(1) > 1 {
        -1
    } else {
        1
    }
}

/// A split asked for so part of a stack can be sold: the new stack is
/// staged when it appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingSplit {
    source: u32,
    weenie_class_id: u32,
    quantity: i32,
    seen_in_flight: bool,
}

/// Where a split for a sale stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitOutcome {
    /// The new stack is in the pack: stage it.
    Arrived(u32),
    /// Still on its way.
    Waiting,
    /// The request ended with no new stack: forget it.
    Failed,
}

impl PendingSplit {
    /// Split `quantity` off `item` so it can be sold: the request to send,
    /// what to tell the player, and the split to follow.
    ///
    /// # Errors
    ///
    /// The refusal's words when no request can go out now.
    pub fn start(
        item: &ClientObject,
        quantity: i32,
        can_make_request: bool,
    ) -> Result<(Self, SessionCommand, String), &'static str> {
        if !can_make_request {
            return Err(CANNOT_SPLIT_TO_SELL_MESSAGE);
        }
        let guid = item.object_id();
        let pending = Self {
            source: guid,
            weenie_class_id: item.weenie_class_id,
            quantity,
            seen_in_flight: false,
        };
        let command = SessionCommand::Item(ItemCommand::MoveToContainer {
            item: ObjectRef::Guid(guid),
            container: ObjectRef::Guid(item.container_id),
            slot: 0,
            amount: quantity.unsigned_abs(),
            join_stack: false,
        });
        let name = if item.name.trim().is_empty() {
            "item"
        } else {
            item.name.as_str()
        };
        Ok((
            pending,
            command,
            format!("Splitting the {name} before selling them"),
        ))
    }

    /// Where the split stands against `items`.
    pub fn follow(&mut self, items: &ItemsView<'_>) -> SplitOutcome {
        let found = items.objects.objects().find(|o| {
            o.object_id() != self.source
                && o.weenie_class_id == self.weenie_class_id
                && o.stack_size == self.quantity
                && items
                    .objects
                    .is_owned_by_object(o.object_id(), items.player)
        });
        if let Some(o) = found {
            return SplitOutcome::Arrived(o.object_id());
        }
        if items.is_pending(self.source) {
            self.seen_in_flight = true;
            SplitOutcome::Waiting
        } else if self.seen_in_flight {
            SplitOutcome::Failed
        } else {
            SplitOutcome::Waiting
        }
    }
}

/// How long a guarded split waits for its new stack, seconds.
pub const SPLIT_WAIT_SECONDS: f64 = 10.0;

/// A split asked for so part of a stack can be sold or offered, made safe
/// against what [`PendingSplit`] leaves to chance: a stack of the same kind
/// and size the player already had is never taken for the new one; the new
/// stack is handed on only once the split's own request is over and
/// another can go out (so the next request is not refused for a busy
/// slot); and a split that never shows is given up after
/// [`SPLIT_WAIT_SECONDS`].
#[derive(Debug, Clone, PartialEq)]
pub struct GuardedSplit {
    split: PendingSplit,
    /// The player's stacks of the kind when the split was asked for.
    known: Vec<u32>,
    /// When it was asked for, seconds on the caller's clock.
    started: f64,
}

impl GuardedSplit {
    /// Split `quantity` off `item` (in `items`) at `now` seconds: the
    /// request to send, what to tell the player, and the split to follow.
    ///
    /// # Errors
    ///
    /// The refusal's words when no request can go out now.
    pub fn start(
        item: &ClientObject,
        quantity: i32,
        items: &ItemsView<'_>,
        now: f64,
    ) -> Result<(Self, SessionCommand, String), &'static str> {
        let (split, command, words) = PendingSplit::start(item, quantity, items.can_make_request)?;
        let known = items
            .objects
            .objects()
            .filter(|o| {
                o.weenie_class_id == item.weenie_class_id
                    && items
                        .objects
                        .is_owned_by_object(o.object_id(), items.player)
            })
            .map(ClientObject::object_id)
            .collect();
        Ok((
            Self {
                split,
                known,
                started: now,
            },
            command,
            words,
        ))
    }

    /// Where the split stands against `items` at `now` seconds.
    pub fn follow(&mut self, items: &ItemsView<'_>, now: f64) -> SplitOutcome {
        let s = self.split;
        let found = items.objects.objects().find(|o| {
            o.object_id() != s.source
                && !self.known.contains(&o.object_id())
                && o.weenie_class_id == s.weenie_class_id
                && o.stack_size == s.quantity
                && items
                    .objects
                    .is_owned_by_object(o.object_id(), items.player)
        });
        let in_flight = items.is_pending(s.source);
        if in_flight {
            self.split.seen_in_flight = true;
        }
        match found {
            Some(o) if !in_flight && items.can_make_request => SplitOutcome::Arrived(o.object_id()),
            Some(_) => SplitOutcome::Waiting,
            None if self.split.seen_in_flight && !in_flight => SplitOutcome::Failed,
            None if now - self.started > SPLIT_WAIT_SECONDS => SplitOutcome::Failed,
            None => SplitOutcome::Waiting,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac_core::items::ClientObjectTable;

    const PLAYER: u32 = 0x5000_0001;
    const ARROWS: u32 = 300;

    fn stack(table: &mut ClientObjectTable, id: u32, size: i32) {
        let mut o = ClientObject::new(id);
        o.weenie_class_id = ARROWS;
        o.stack_size = size;
        o.container_id = PLAYER;
        table.add_or_update(o, &mut Vec::new());
    }

    fn view<'a>(objects: &'a ClientObjectTable, pending: &'a [u32], free: bool) -> ItemsView<'a> {
        ItemsView {
            pending_sources: pending,
            can_make_request: free,
            ..ItemsView::new(objects, PLAYER)
        }
    }

    // A stack of the same kind and size the player already carried (the
    // wielded arrows) is not the new one; the new one is handed on once
    // the split's request is over and another may go out.
    #[test]
    fn only_a_new_stack_counts_and_only_once_requests_are_free() {
        let mut table = ClientObjectTable::new();
        stack(&mut table, 1, 100);
        stack(&mut table, 2, 10);
        let source = table.get(1).cloned().expect("the stack");
        let (mut split, _, _) =
            GuardedSplit::start(&source, 10, &view(&table, &[], true), 0.0).expect("asked");
        assert_eq!(
            split.follow(&view(&table, &[1], false), 0.1),
            SplitOutcome::Waiting
        );
        stack(&mut table, 3, 10);
        assert_eq!(
            split.follow(&view(&table, &[1], false), 0.2),
            SplitOutcome::Waiting
        );
        assert_eq!(
            split.follow(&view(&table, &[], false), 0.3),
            SplitOutcome::Waiting
        );
        assert_eq!(
            split.follow(&view(&table, &[], true), 0.4),
            SplitOutcome::Arrived(3)
        );
    }

    // A split never seen in flight, with nothing new, gives up in time.
    #[test]
    fn a_split_that_never_shows_is_given_up() {
        let mut table = ClientObjectTable::new();
        stack(&mut table, 1, 100);
        let source = table.get(1).cloned().expect("the stack");
        let (mut split, _, _) =
            GuardedSplit::start(&source, 10, &view(&table, &[], true), 0.0).expect("asked");
        assert_eq!(
            split.follow(&view(&table, &[], true), 5.0),
            SplitOutcome::Waiting
        );
        assert_eq!(
            split.follow(&view(&table, &[], true), 10.5),
            SplitOutcome::Failed
        );
    }
}
