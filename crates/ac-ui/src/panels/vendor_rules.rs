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
