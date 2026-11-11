use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::mask::LayerMask;
use crate::ops::{CropRotateParams, EditOp};

pub const DEFAULT_MAX_HISTORY: usize = 100;

/// An individual non-destructive operation node within the edit stack.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EditNode {
    pub id: Uuid,
    pub enabled: bool,
    pub opacity: f32, // 0.0..=1.0
    pub op: EditOp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask: Option<LayerMask>,
}

impl EditNode {
    pub fn new(op: EditOp) -> Self {
        Self {
            id: Uuid::new_v4(),
            enabled: true,
            opacity: 1.0,
            op,
            mask: None,
        }
    }

    pub fn with_mask(op: EditOp, mask: LayerMask) -> Self {
        Self {
            id: Uuid::new_v4(),
            enabled: true,
            opacity: 1.0,
            op,
            mask: Some(mask),
        }
    }
}

/// The non-destructive stack of edits applied to an image.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EditStack {
    pub version: u32,
    pub nodes: Vec<EditNode>,
    pub crop: CropRotateParams,
}

impl Default for EditStack {
    fn default() -> Self {
        Self {
            version: 1,
            nodes: Vec::new(),
            crop: CropRotateParams::default(),
        }
    }
}

impl EditStack {
    /// Append a new operation to the top of the stack.
    pub fn push_op(&mut self, op: EditOp) -> Uuid {
        if let EditOp::CropRotate(ref crop) = op {
            self.crop = crop.clone();
        }
        let node = EditNode::new(op);
        let id = node.id;
        self.nodes.push(node);
        id
    }

    /// Insert an operation at a specific index in the stack.
    pub fn insert_op(&mut self, index: usize, op: EditOp) -> Uuid {
        if let EditOp::CropRotate(ref crop) = op {
            self.crop = crop.clone();
        }
        let node = EditNode::new(op);
        let id = node.id;
        let idx = index.min(self.nodes.len());
        self.nodes.insert(idx, node);
        id
    }

    /// Update an existing node's operation by UUID.
    pub fn update_node(&mut self, id: Uuid, op: EditOp) -> bool {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            if let EditOp::CropRotate(ref crop) = op {
                self.crop = crop.clone();
            }
            node.op = op;
            true
        } else {
            false
        }
    }

    /// Set a node's opacity by UUID.
    pub fn set_node_opacity(&mut self, id: Uuid, opacity: f32) -> bool {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.opacity = opacity.clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }

    /// Set or clear a node's mask by UUID.
    pub fn set_node_mask(&mut self, id: Uuid, mask: Option<LayerMask>) -> bool {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.mask = mask;
            true
        } else {
            false
        }
    }

    /// Look up a node reference by UUID.
    pub fn node_by_id(&self, id: Uuid) -> Option<&EditNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Look up a mutable node reference by UUID.
    pub fn node_by_id_mut(&mut self, id: Uuid) -> Option<&mut EditNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Toggle the enabled/disabled state of a node by UUID.
    pub fn toggle_node(&mut self, id: Uuid) -> Option<bool> {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            node.enabled = !node.enabled;
            Some(node.enabled)
        } else {
            None
        }
    }

    /// Remove a node from the stack by UUID.
    pub fn remove_node(&mut self, id: Uuid) -> Option<EditNode> {
        if let Some(idx) = self.nodes.iter().position(|n| n.id == id) {
            Some(self.nodes.remove(idx))
        } else {
            None
        }
    }

    /// Move a node from one index to another (reordering edits).
    pub fn move_node(&mut self, from_idx: usize, to_idx: usize) -> bool {
        if from_idx >= self.nodes.len() || to_idx >= self.nodes.len() {
            return false;
        }
        let node = self.nodes.remove(from_idx);
        self.nodes.insert(to_idx, node);
        true
    }

    /// Iterate over active (enabled) nodes up to an optional limit.
    /// Powers Snapseed's "View Edits" step-by-step preview.
    pub fn active_ops_up_to(&self, limit: Option<usize>) -> impl Iterator<Item = &EditNode> {
        let count = limit.unwrap_or(self.nodes.len());
        self.nodes.iter().take(count).filter(|n| n.enabled)
    }

    /// Check if the stack has no active modifications.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.crop.is_identity()
    }
}

/// Generic undo/redo history manager for state management.
/// Used for both the document `EditStack` and in-tool live editing sessions (`ToolSessionHistory`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HistoryManager<T: Clone> {
    current: T,
    undo_stack: Vec<T>,
    redo_stack: Vec<T>,
    max_entries: usize,
}

impl<T: Clone> HistoryManager<T> {
    pub fn new(initial: T) -> Self {
        Self::with_capacity(initial, DEFAULT_MAX_HISTORY)
    }

    pub fn with_capacity(initial: T, max_entries: usize) -> Self {
        Self {
            current: initial,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_entries: max_entries.max(1),
        }
    }

    /// Commit a new state to history, clearing the redo stack.
    pub fn commit(&mut self, state: T) {
        let old = std::mem::replace(&mut self.current, state);
        self.undo_stack.push(old);
        if self.undo_stack.len() > self.max_entries {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Undo to the previous state, pushing current onto the redo stack.
    pub fn undo(&mut self) -> Option<&T> {
        if let Some(prev) = self.undo_stack.pop() {
            let old = std::mem::replace(&mut self.current, prev);
            self.redo_stack.push(old);
            Some(&self.current)
        } else {
            None
        }
    }

    /// Redo to the next state, pushing current onto the undo stack.
    pub fn redo(&mut self) -> Option<&T> {
        if let Some(next) = self.redo_stack.pop() {
            let old = std::mem::replace(&mut self.current, next);
            self.undo_stack.push(old);
            Some(&self.current)
        } else {
            None
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn current(&self) -> &T {
        &self.current
    }

    pub fn current_mut(&mut self) -> &mut T {
        &mut self.current
    }
}

/// Snapseed 4.1 in-tool interactive session history for gestures and sliders.
pub type ToolSessionHistory = HistoryManager<EditOp>;
