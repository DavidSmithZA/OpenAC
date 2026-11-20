//! What an item let go on a pack does: the inventory's drop rules, apart
//! from any window. A drop lands on a pack's grid (left of the item it was
//! let go on), on a side pack's icon, or on the main pack's icon. It may
//! join a stack, merge with the stack it lands on, find another pack with
//! room when the one named is full, or be refused with a notice.

use ac_core::items::placement::{
    choose_container, empty_item_slots, evaluate_container_placement, is_pack,
    item_list_drop_placement, placement_rejection_text,
};
use ac_core::items::{ClientObject, EquipMask, PlacementRejection, StackMergeItem, StackMergePlan};
use ac_runtime::inventory::{ItemCommand, ObjectRef, placement_conflict_text, plan_join_stack};

use crate::event::{DragAcceptance, ItemDragPayload, ItemDragSource};
use crate::panels::items::ItemsView;

/// Where an item is let go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropSpot {
    /// A cell of the grid of pack `open` (the pack shown open).
    Grid {
        /// The pack whose grid it is.
        open: u32,
    },
    /// A side pack's icon in the pack column (a pack dropped there goes
    /// among the packs, at that place).
    SidePack,
    /// The main pack's icon (a pack dropped there goes first).
    MainPack,
}

/// What a drop does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropAnswer {
    /// Nothing at all.
    Nothing,
    /// Tell the player why not.
    Notice(String),
    /// Send `command`, and select `select` when it names one.
    Send {
        /// The request.
        command: ItemCommand,
        /// The item to select after it (a joined stack).
        select: Option<u32>,
    },
}

fn notice(view: &ItemsView<'_>, r: PlacementRejection, item: u32, dest: u32) -> DropAnswer {
    DropAnswer::Notice(placement_rejection_text(
        r,
        view.get(item),
        view.get(dest),
        view.player,
    ))
}

fn is_capacity_rejection(r: Option<PlacementRejection>) -> bool {
    matches!(
        r,
        Some(PlacementRejection::ItemCapacityFull | PlacementRejection::ContainerCapacityFull)
    )
}

fn pack_ref(view: &ItemsView<'_>, container: u32) -> ObjectRef {
    if container == view.player {
        ObjectRef::Player
    } else {
        ObjectRef::Guid(container)
    }
}

/// How much of `item` a drag moves, and whether that is the whole stack:
/// the selected stack's split quantity splits it, wherever it comes from.
/// Part of a stack needs a slot of its own, even in the pack that holds the
/// rest.
#[must_use]
pub fn moving_amount(view: &ItemsView<'_>, selection: Option<u32>, item: u32) -> (u32, bool) {
    let full = view
        .get(item)
        .map_or(1, |d| d.stack_size.max(1).unsigned_abs());
    let split = view.split_size(item, full, selection);
    (split, split >= full)
}

/// Whether `item` may go where a drop on `target_item` at `spot` puts it:
/// (refusal, the item is a pack, the destination).
fn evaluate(
    view: &ItemsView<'_>,
    selection: Option<u32>,
    spot: DropSpot,
    target_item: u32,
    item: u32,
) -> (Option<PlacementRejection>, bool, u32) {
    let source_is_bag = view.get(item).is_some_and(is_pack);
    if item == 0 {
        return (Some(PlacementRejection::InvalidItem), source_is_bag, 0);
    }
    let destination;
    match spot {
        DropSpot::Grid { open } => {
            destination = open;
            if source_is_bag {
                return (
                    Some(PlacementRejection::ContainerCapacityFull),
                    source_is_bag,
                    destination,
                );
            }
        }
        DropSpot::SidePack | DropSpot::MainPack => {
            destination = if source_is_bag {
                view.player
            } else {
                target_item
            };
            if !source_is_bag && (target_item == 0 || target_item == item) {
                return (
                    Some(PlacementRejection::InvalidDestination),
                    source_is_bag,
                    destination,
                );
            }
        }
    }
    let whole_stack = moving_amount(view, selection, item).1;
    (
        evaluate_container_placement(view.objects, item, destination, view.player, whole_stack)
            .err(),
        source_is_bag,
        destination,
    )
}

/// Whether a drop of `p` on `target_item` at `spot` would be taken (what a
/// cell shows while the drag hovers it).
#[must_use]
pub fn acceptance(
    view: &ItemsView<'_>,
    selection: Option<u32>,
    spot: DropSpot,
    target_item: u32,
    p: &ItemDragPayload,
) -> DragAcceptance {
    if p.source_kind.is_alias() {
        return DragAcceptance::None;
    }
    let (legality, source_is_bag, destination) =
        evaluate(view, selection, spot, target_item, p.obj_id);
    // Over a pack's own icon (a side pack or the main pack) the answer is
    // that pack's free item slots alone: none refuses, even for a stack
    // already inside it, and even though letting go there still finds the
    // item a place in another pack. Something in the side column that
    // carries nothing, a focus for one, has none.
    let over_a_pack_icon =
        !source_is_bag && matches!(spot, DropSpot::SidePack | DropSpot::MainPack);
    if over_a_pack_icon {
        return if target_item != 0 && empty_item_slots(view.objects, target_item) != 0 {
            DragAcceptance::Accept
        } else {
            DragAcceptance::Reject
        };
    }
    // Anywhere else a full pack is no refusal while the drop has somewhere
    // to go, found by the same search the drop runs.
    let mut legality = legality;
    if is_capacity_rejection(legality)
        && choose_container(
            view.objects,
            p.obj_id,
            destination,
            destination,
            view.player,
            moving_amount(view, selection, p.obj_id).1,
        )
        .is_ok()
    {
        legality = None;
    }
    if legality.is_none() {
        DragAcceptance::Accept
    } else {
        DragAcceptance::Reject
    }
}

/// The place a drop on a cell of pack `container`'s own lists asks for: the
/// loose-item grid (`bags` false) or the pack column (true). `None` when
/// the drop leaves the item where it is.
#[must_use]
pub fn list_drop_placement(
    view: &ItemsView<'_>,
    container: u32,
    item: u32,
    bags: bool,
    target_item: u32,
    target_index: i32,
    whole_stack: bool,
) -> Option<i32> {
    let listed: Vec<u32> = view
        .objects
        .get_contents(container)
        .iter()
        .copied()
        .filter(|&g| {
            view.get(g).is_some_and(|o| {
                o.currently_equipped_location == EquipMask::NONE && is_pack(o) == bags
            })
        })
        .collect();
    let source = listed.iter().position(|&g| g == item);
    // An empty cell is past the last item.
    let drop_index = if target_item != 0 {
        target_index
    } else {
        i32::MAX
    };
    item_list_drop_placement(drop_index, listed.len(), source, whole_stack)
}

/// What letting `p` go on `target_item` (cell `target_index`; 0 and -1 for
/// an empty cell) at `spot` does. `selection` is the selected item (its
/// split quantity splits a drag of it).
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn answer(
    view: &ItemsView<'_>,
    selection: Option<u32>,
    spot: DropSpot,
    target_item: u32,
    target_index: i32,
    p: &ItemDragPayload,
) -> DropAnswer {
    if p.source_kind.is_alias() {
        return DropAnswer::Nothing;
    }
    let item = p.obj_id;
    if item == 0 {
        return DropAnswer::Nothing;
    }
    let pickup = p.source_kind == ItemDragSource::Ground;
    let (mut legality, _, mut destination) = evaluate(view, selection, spot, target_item, item);
    if (legality.is_none() || is_capacity_rejection(legality))
        && let Some(join) = join_stack_in_pack(view, selection, spot, target_item, item, pickup)
    {
        return join;
    }
    let mut fallthrough = 0;
    if is_capacity_rejection(legality) {
        // The search starts where the drop names: a pack's icon looks in
        // that pack and the packs inside it, the grid in the pack shown open
        // (the whole inventory when that is the main pack, nothing more for
        // a side pack, which holds no packs).
        let root = destination;
        let whole_stack = moving_amount(view, selection, item).1;
        match choose_container(
            view.objects,
            item,
            root,
            destination,
            view.player,
            whole_stack,
        ) {
            Ok(c) => fallthrough = c,
            Err(no_room) => {
                // Nothing has room: the notice names where it looked.
                return notice(view, no_room, item, root);
            }
        }
        legality =
            evaluate_container_placement(view.objects, item, fallthrough, view.player, whole_stack)
                .err();
        destination = fallthrough;
    }
    if let Some(r) = legality {
        return notice(view, r, item, destination);
    }
    // An item already on its way into the open pack: the grid takes no
    // second one until the server answers.
    if let DropSpot::Grid { open } = spot
        && let Some(pending) = view.pending_placement
        && pending.container_id == open
    {
        let name = view
            .get(pending.item_id)
            .map(|o| o.name.as_str())
            .filter(|n| !n.trim().is_empty())
            .unwrap_or("that item");
        return DropAnswer::Notice(placement_conflict_text(name));
    }
    if !view.can_make_request {
        return DropAnswer::Notice(ac_runtime::inventory::REQUEST_BUSY.to_owned());
    }
    if let DropSpot::Grid { open } = spot
        && target_item != 0
        && let Some(merge) = merge(
            view,
            selection,
            p,
            open,
            target_item,
            target_index,
            fallthrough,
        )
    {
        return merge;
    }
    let dragged = view.get(item);
    let source_is_bag = dragged.is_some_and(is_pack);
    let (split, whole_stack) = moving_amount(view, selection, item);
    let mut container;
    let placement: Option<i32>;
    match spot {
        DropSpot::Grid { open } => {
            container = open;
            // Lands left of the hovered item, wherever it came from: this
            // pack, another pack, the paperdoll or the ground.
            placement = list_drop_placement(
                view,
                container,
                item,
                false,
                target_item,
                target_index,
                whole_stack,
            );
        }
        DropSpot::SidePack | DropSpot::MainPack => {
            if source_is_bag {
                container = view.player;
                if container == 0 {
                    return DropAnswer::Nothing;
                }
                placement = if spot == DropSpot::MainPack {
                    Some(0)
                } else {
                    list_drop_placement(
                        view,
                        container,
                        item,
                        true,
                        target_item,
                        target_index,
                        whole_stack,
                    )
                };
            } else {
                if target_item == 0 || target_item == item {
                    return DropAnswer::Nothing;
                }
                // A full pack was a capacity refusal, answered by the
                // search above; one that takes it here has room.
                container = target_item;
                placement = Some(0);
            }
        }
    }
    // The pack the search found takes the item at the slot this drop
    // named: the grid's drop slot, or the top for a pack's icon.
    if fallthrough != 0 {
        container = fallthrough;
    }
    let Some(placement) = placement else {
        return DropAnswer::Nothing; // the item is already there
    };
    if container == item {
        return DropAnswer::Nothing;
    }
    DropAnswer::Send {
        command: ItemCommand::DropInto {
            item: ObjectRef::Guid(item),
            container: pack_ref(view, container),
            slot: placement,
            amount: if whole_stack { 0 } else { split },
            pickup,
            join_stack: false,
        },
        select: None,
    }
}

/// A drop on a pack's icon (a side pack or the main pack) first joins the
/// moving amount to the first stack in that pack, or in a pack inside it,
/// with room for all of it, and selects that stack. Joining takes no slot,
/// so this comes before any room check. A drop on a cell of a grid never
/// searches like this.
fn join_stack_in_pack(
    view: &ItemsView<'_>,
    selection: Option<u32>,
    spot: DropSpot,
    pack: u32,
    item: u32,
    pickup: bool,
) -> Option<DropAnswer> {
    if matches!(spot, DropSpot::Grid { .. }) {
        return None;
    }
    if pack == 0 || pack == item || view.get(pack).is_none_or(|c| c.items_capacity <= 0) {
        return None;
    }
    let dragged = view.get(item).filter(|d| !is_pack(d))?;
    let full = dragged.stack_size.max(1).unsigned_abs();
    let split = view.split_size(item, full, selection);
    let join = plan_join_stack(view.objects, item, pack, split, view.can_make_request)?;
    Some(DropAnswer::Send {
        command: ItemCommand::DropInto {
            item: ObjectRef::Guid(item),
            container: pack_ref(view, pack),
            slot: 0,
            amount: if split >= full { 0 } else { split },
            pickup,
            join_stack: true,
        },
        select: Some(join.target_object_id),
    })
}

/// A drop on a stack in a grid: a legal merge takes the drop. The session
/// merges, or (the merge no longer legal by its own copy of the stacks)
/// makes the plain drop this one would have been: into the pack shown
/// open, left of the stack dropped on, or the pack with room.
fn merge(
    view: &ItemsView<'_>,
    selection: Option<u32>,
    p: &ItemDragPayload,
    open: u32,
    target: u32,
    target_index: i32,
    fallthrough: u32,
) -> Option<DropAnswer> {
    let source = p.obj_id;
    let (s, t) = (view.get(source)?, view.get(target)?);
    let requested = match view.split {
        Some(q) if selection == Some(source) => i32::try_from(q.value()).unwrap_or(i32::MAX),
        _ => s.stack_size.max(1),
    };
    let item = |o: &ClientObject| StackMergeItem {
        object_id: o.object_id(),
        weenie_class_id: o.weenie_class_id,
        stack_size: o.stack_size,
        max_stack_size: o.stack_size_max,
        trade_state: o.trade_state,
    };
    let plan = StackMergePlan::plan(&item(s), &item(t), view.can_make_request, requested)?;
    let (split, whole_stack) = moving_amount(view, selection, source);
    // As the plain grid drop: left of the stack dropped on, in the pack
    // shown open or, when it has no room, the pack the search found.
    let slot = list_drop_placement(view, open, source, false, target, target_index, whole_stack);
    let container = if fallthrough != 0 { fallthrough } else { open };
    Some(DropAnswer::Send {
        command: ItemCommand::DropOnStack {
            item: ObjectRef::Guid(plan.source_object_id),
            target: ObjectRef::Guid(plan.target_object_id),
            merge_amount: plan.amount,
            container: pack_ref(view, container),
            slot: slot.filter(|_| container != source),
            amount: if whole_stack { 0 } else { split },
            pickup: p.source_kind == ItemDragSource::Ground,
        },
        select: Some(plan.target_object_id),
    })
}
