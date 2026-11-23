//! What any salvage window stages: the salvaging tool it is open on, the
//! items to salvage in staging order, and the one material allowed when the
//! player's options allow only one at a time. A pack stages its suitable
//! contents. The staging follows the object table: a staged item gone, no
//! longer the player's or no longer suitable leaves it; the tool gone
//! closes it. The staged items are marked in the session while staged.

use std::collections::HashSet;

use ac_core::items::{ClientObject, ItemType, PublicWeenieFlags, is_salvage_suitable};
use ac_runtime::inventory::{ItemCommand, ObjectRef};
use ac_runtime::session::SessionCommand;

use crate::event::{ItemDragPayload, ItemDragSource};
use crate::panels::items::ItemsView;

/// What a refused list says.
pub const INVALID_LIST: &str = "The list of items you are attempting to salvage is invalid.";
/// What a send that cannot be made says.
pub const CANNOT_SALVAGE_NOW: &str = "You cannot salvage those items right now.";

/// See the module documentation.
/// Why an item cannot go on the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SalvageRefusal {
    /// The list has no salvaging tool the player carries.
    NoTool,
    /// It is the tool the list salvages with.
    IsTheTool,
    /// The player does not carry it.
    NotCarried,
    /// It is on the list already.
    AlreadyListed,
    /// It is not in the object table.
    Gone,
    /// It is retained.
    Retained,
    /// It could be salvaged, but only one material goes at a time and the
    /// list holds another.
    OtherMaterial {
        /// Its material.
        item: u32,
        /// The list's.
        listed: u32,
    },
    /// It cannot be salvaged at all.
    Unsuitable,
}

/// Whether a drag can stage its item at all: only what is carried in the
/// packs comes in (worn gear and things on the ground do not).
#[must_use]
pub fn drag_stages(p: &ItemDragPayload) -> bool {
    p.source_kind == ItemDragSource::Inventory && p.obj_id != 0
}

/// Why `item` itself cannot be salvaged with `material` on the list (0:
/// none yet), if it cannot.
#[must_use]
pub fn item_refusal(
    item: &ClientObject,
    material: u32,
    allow_multiple: bool,
) -> Option<SalvageRefusal> {
    if is_salvage_suitable(item, material, allow_multiple) {
        None
    } else if item.flags().contains(PublicWeenieFlags::RETAINED) {
        Some(SalvageRefusal::Retained)
    } else if is_salvage_suitable(item, 0, true) {
        Some(SalvageRefusal::OtherMaterial {
            item: item.material_type.unwrap_or(0),
            listed: material,
        })
    } else {
        Some(SalvageRefusal::Unsuitable)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SalvageList {
    tool: u32,
    material: u32,
    items: Vec<u32>,
    /// The staged items the session was last told to mark.
    marked: Vec<u32>,
}

fn is_owned(view: &ItemsView<'_>, id: u32) -> bool {
    view.objects.is_owned_by_object(id, view.player)
}

fn still_valid(view: &ItemsView<'_>, id: u32) -> bool {
    is_owned(view, id) && view.get(id).is_some()
}

impl SalvageList {
    /// The tool it is open on (0: none).
    #[must_use]
    pub fn tool(&self) -> u32 {
        self.tool
    }

    /// The staged items, in staging order.
    #[must_use]
    pub fn items(&self) -> &[u32] {
        &self.items
    }

    /// The staged material (0: none yet).
    #[must_use]
    pub fn material(&self) -> u32 {
        self.material
    }

    /// Whether `tool` is a salvaging tool the player carries.
    #[must_use]
    pub fn is_tool_available(tool: u32, view: &ItemsView<'_>) -> bool {
        tool != 0
            && is_owned(view, tool)
            && view
                .get(tool)
                .is_some_and(|t| t.item_type.intersects(ItemType::TINKERING_TOOL))
    }

    /// Open on `tool` (a salvaging tool the player carries), the staging
    /// empty; false when it is not one.
    pub fn open(&mut self, tool: u32, view: &ItemsView<'_>) -> bool {
        if !Self::is_tool_available(tool, view) {
            return false;
        }
        self.clear();
        self.tool = tool;
        true
    }

    /// `item` may be staged: it is open on a usable tool, and the item is
    /// the player's, not the tool, not staged yet, and either a container
    /// holding items (packs in it do not count) or suitable for salvage (of
    /// the staged material when one material at a time is allowed).
    #[must_use]
    pub fn can_add(&self, item: u32, view: &ItemsView<'_>, allow_multiple: bool) -> bool {
        self.refusal(item, view, allow_multiple).is_none()
    }

    /// Why `item` cannot go on the list, if it cannot (a pack with
    /// contents can: its suitable contents go on).
    #[must_use]
    pub fn refusal(
        &self,
        item: u32,
        view: &ItemsView<'_>,
        allow_multiple: bool,
    ) -> Option<SalvageRefusal> {
        if !Self::is_tool_available(self.tool, view) {
            return Some(SalvageRefusal::NoTool);
        }
        if item == self.tool {
            return Some(SalvageRefusal::IsTheTool);
        }
        if !is_owned(view, item) {
            return Some(SalvageRefusal::NotCarried);
        }
        if self.items.contains(&item) {
            return Some(SalvageRefusal::AlreadyListed);
        }
        let Some(object) = view.get(item) else {
            return Some(SalvageRefusal::Gone);
        };
        if !view.objects.get_contained_items(item).is_empty() {
            return None;
        }
        item_refusal(object, self.material, allow_multiple)
    }

    /// Stage `item` (a pack stages its suitable contents); the notices to
    /// show ("Adding contents of ..."), or `None` when it may not be
    /// staged.
    pub fn add(
        &mut self,
        item: u32,
        view: &ItemsView<'_>,
        allow_multiple: bool,
    ) -> Option<Vec<String>> {
        if !self.can_add(item, view, allow_multiple) {
            return None;
        }
        let mut visited = HashSet::new();
        let mut notices = Vec::new();
        self.add_recursive(item, view, allow_multiple, &mut visited, &mut notices);
        Some(notices)
    }

    fn add_recursive(
        &mut self,
        item: u32,
        view: &ItemsView<'_>,
        allow_multiple: bool,
        visited: &mut HashSet<u32>,
        notices: &mut Vec<String>,
    ) {
        if !visited.insert(item) || !self.can_add(item, view, allow_multiple) {
            return;
        }
        // A dropped container contributes its item list only. Its packs are
        // a separate list and are not walked, so the main pack adds the
        // character's own items and leaves every side pack alone.
        let contents = view.objects.get_contained_items(item);
        if !contents.is_empty() {
            if let Some(pack) = view.get(item) {
                notices.push(format!("Adding contents of {}", pack.appropriate_name()));
            }
            for child in contents {
                self.add_recursive(child, view, allow_multiple, visited, notices);
            }
            return;
        }
        let Some(object) = view.get(item) else {
            return;
        };
        self.items.push(item);
        if self.items.len() == 1 {
            self.material = object.material_type.unwrap_or(0);
        }
    }

    /// Take `item` out.
    pub fn remove(&mut self, item: u32) {
        self.items.retain(|id| *id != item);
        if self.items.is_empty() {
            self.material = 0;
        }
    }

    /// Empty the staging.
    pub fn clear(&mut self) {
        self.items.clear();
        self.material = 0;
    }

    /// Close: no tool, nothing staged.
    pub fn forget(&mut self) {
        self.tool = 0;
        self.clear();
    }

    /// Follow `view`: false when the tool is gone (the caller closes);
    /// otherwise what is no longer valid leaves the staging.
    pub fn follow(&mut self, view: &ItemsView<'_>) -> bool {
        if !Self::is_tool_available(self.tool, view) {
            return false;
        }
        self.items.retain(|&id| {
            still_valid(view, id)
                && view
                    .get(id)
                    .is_some_and(|o| is_salvage_suitable(o, 0, true))
        });
        if self.items.is_empty() {
            self.material = 0;
        }
        true
    }

    /// The salvage request for the staged items, last staged first, the
    /// staging emptied; nothing staged or no tool gives `Ok(None)`.
    ///
    /// # Errors
    ///
    /// The refusal's words: a staged item is no longer valid, or no
    /// request can go out now.
    pub fn salvage(
        &mut self,
        view: &ItemsView<'_>,
    ) -> Result<Option<SessionCommand>, &'static str> {
        if self.items.is_empty() || !Self::is_tool_available(self.tool, view) {
            return Ok(None);
        }
        let invalid = self.items.iter().any(|&id| {
            !still_valid(view, id)
                || !view
                    .get(id)
                    .is_some_and(|o: &ClientObject| is_salvage_suitable(o, 0, true))
        });
        if invalid {
            return Err(INVALID_LIST);
        }
        if !view.can_make_request {
            return Err(CANNOT_SALVAGE_NOW);
        }
        let items: Vec<ObjectRef> = self
            .items
            .iter()
            .rev()
            .map(|id| ObjectRef::Guid(*id))
            .collect();
        self.clear();
        Ok(Some(SessionCommand::Item(ItemCommand::Salvage {
            tool: ObjectRef::Guid(self.tool),
            items,
        })))
    }

    /// The session's marks, when the staging changed since they were last
    /// sent: the staged items are marked as in a trade while staged (moves,
    /// uses and merges refuse them; the inventory draws them so).
    pub fn marks(&mut self) -> Option<SessionCommand> {
        if self.marked == self.items {
            return None;
        }
        self.marked.clone_from(&self.items);
        Some(SessionCommand::Item(ItemCommand::MarkSalvage(
            self.marked.clone(),
        )))
    }
}
