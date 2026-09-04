//! Retained semantic element tree, accessibility metadata, layout, and damage.

use super::geometry::{Insets, Rect};
use super::input::ElementId;

pub const MAX_ELEMENTS: usize = 128;
pub const MAX_DAMAGE_REGIONS: usize = 24;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ElementKind {
    Root,
    Panel,
    Label,
    Image,
    Icon,
    Button,
    TextField,
    PasswordField,
    UserPicker,
    Menu,
    MenuItem,
    Toolbar,
    Dock,
    Window,
    Dialog,
    List,
    ListItem,
    Progress,
    Toggle,
    Slider,
    SplitView,
    StatusItem,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AccessibilityRole {
    Application,
    Group,
    Text,
    Image,
    Button,
    TextInput,
    PasswordInput,
    List,
    ListItem,
    Menu,
    MenuItem,
    Window,
    Dialog,
    Progress,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ElementState(pub u16);

impl ElementState {
    pub const ENABLED: u16 = 1;
    pub const FOCUSED: u16 = 2;
    pub const HOVERED: u16 = 4;
    pub const PRESSED: u16 = 8;
    pub const SELECTED: u16 = 16;
    pub const SECURE: u16 = 32;

    // ------------------------=
    // FUNC: contains
    // DESC: Tests one semantic state bit without exposing widget implementation.
    // ------------------=
    pub const fn contains(self, flag: u16) -> bool {
        self.0 & flag != 0
    }
}

#[derive(Clone, Copy)]
pub struct SemanticElement {
    pub id: ElementId,
    pub parent: Option<ElementId>,
    pub kind: ElementKind,
    pub accessibility_role: AccessibilityRole,
    pub bounds: Rect,
    pub previous_bounds: Rect,
    pub state: ElementState,
    pub label_id: u32,
    pub action_mask: u16,
    pub z_order: i16,
    pub visible: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SceneError {
    Full,
    Duplicate,
    MissingParent,
    InvalidGeometry,
}

pub struct UiScene {
    elements: [Option<SemanticElement>; MAX_ELEMENTS],
    count: u8,
    frame_sequence: u64,
}

impl UiScene {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty retained semantic scene with bounded storage.
    // ------------------=
    pub const fn new() -> Self {
        Self { elements: [None; MAX_ELEMENTS], count: 0, frame_sequence: 0 }
    }

    // ------------------------=
    // FUNC: begin_frame
    // DESC: Opens a retained-tree mutation transaction for a numbered frame.
    // ------------------=
    pub fn begin_frame(&mut self, sequence: u64) {
        self.frame_sequence = sequence;
        for element in self.elements.iter_mut().flatten() {
            element.previous_bounds = element.bounds;
        }
    }

    // ------------------------=
    // FUNC: insert
    // DESC: Adds one typed semantic element after validating identity and parentage.
    // ------------------=
    pub fn insert(&mut self, element: SemanticElement) -> Result<(), SceneError> {
        if element.bounds.width == 0 || element.bounds.height == 0 {
            return Err(SceneError::InvalidGeometry);
        }
        if self.get(element.id).is_some() {
            return Err(SceneError::Duplicate);
        }
        if let Some(parent) = element.parent {
            if self.get(parent).is_none() {
                return Err(SceneError::MissingParent);
            }
        }
        let slot = self.elements.iter_mut().find(|entry| entry.is_none()).ok_or(SceneError::Full)?;
        *slot = Some(element);
        self.count = self.count.saturating_add(1);
        Ok(())
    }

    // ------------------------=
    // FUNC: get
    // DESC: Resolves an immutable retained element by stable scene identity.
    // ------------------=
    pub fn get(&self, id: ElementId) -> Option<&SemanticElement> {
        self.elements.iter().flatten().find(|element| element.id == id)
    }

    // ------------------------=
    // FUNC: get_mut
    // DESC: Resolves a mutable retained element inside the current frame transaction.
    // ------------------=
    pub fn get_mut(&mut self, id: ElementId) -> Option<&mut SemanticElement> {
        self.elements.iter_mut().flatten().find(|element| element.id == id)
    }

    // ------------------------=
    // FUNC: hit_test
    // DESC: Finds the highest visible actionable element at a pointer location.
    // ------------------=
    pub fn hit_test(&self, point: super::geometry::Point) -> Option<ElementId> {
        let mut result = None;
        let mut highest = i16::MIN;
        for element in self.elements.iter().flatten() {
            if element.visible && element.action_mask != 0 && element.bounds.contains(point) && element.z_order >= highest {
                result = Some(element.id);
                highest = element.z_order;
            }
        }
        result
    }

    // ------------------------=
    // FUNC: commit_frame
    // DESC: Computes old-and-new geometry damage for every changed retained element.
    // ------------------=
    pub fn commit_frame(&self, damage: &mut DamageTracker) {
        for element in self.elements.iter().flatten() {
            if element.bounds != element.previous_bounds {
                damage.add(element.previous_bounds);
                damage.add(element.bounds);
            }
        }
    }

    // ------------------------=
    // FUNC: count
    // DESC: Returns the current bounded semantic element count.
    // ------------------=
    pub const fn count(&self) -> u8 {
        self.count
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    Horizontal,
    Vertical,
}

// ------------------------=
// FUNC: linear_layout
// DESC: Divides a container into deterministic row or column cells with spacing and insets.
// ------------------=
pub fn linear_layout(container: Rect, insets: Insets, axis: Axis, count: usize, spacing: u16, out: &mut [Rect]) -> usize {
    let usable = container.inset(insets);
    let count = count.min(out.len());
    if count == 0 {
        return 0;
    }
    let gaps = spacing as u32 * count.saturating_sub(1) as u32;
    let extent = match axis { Axis::Horizontal => usable.width, Axis::Vertical => usable.height };
    let cell = extent.saturating_sub(gaps) / count as u32;
    for (index, rect) in out.iter_mut().take(count).enumerate() {
        let offset = index as u32 * (cell + spacing as u32);
        *rect = match axis {
            Axis::Horizontal => Rect { x: usable.x + offset as i32, y: usable.y, width: cell, height: usable.height },
            Axis::Vertical => Rect { x: usable.x, y: usable.y + offset as i32, width: usable.width, height: cell },
        };
    }
    count
}

pub struct DamageTracker {
    regions: [Rect; MAX_DAMAGE_REGIONS],
    count: u8,
    full_redraw: bool,
}

impl DamageTracker {
    // ------------------------=
    // FUNC: new
    // DESC: Creates an empty bounded damage list for atomic frame presentation.
    // ------------------=
    pub const fn new() -> Self {
        Self { regions: [Rect { x: 0, y: 0, width: 0, height: 0 }; MAX_DAMAGE_REGIONS], count: 0, full_redraw: false }
    }

    // ------------------------=
    // FUNC: begin_frame
    // DESC: Clears damage accumulated by the previously presented frame.
    // ------------------=
    pub fn begin_frame(&mut self) {
        self.count = 0;
        self.full_redraw = false;
    }

    // ------------------------=
    // FUNC: add
    // DESC: Adds or merges one damaged rectangle and safely collapses on overflow.
    // ------------------=
    pub fn add(&mut self, rect: Rect) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }
        for region in self.regions[..self.count as usize].iter_mut() {
            if region.intersects(rect) {
                *region = region.union(rect);
                return;
            }
        }
        if self.count as usize == MAX_DAMAGE_REGIONS {
            let mut combined = rect;
            for region in &self.regions[..self.count as usize] {
                combined = combined.union(*region);
            }
            self.regions[0] = combined;
            self.count = 1;
            self.full_redraw = true;
            return;
        }
        self.regions[self.count as usize] = rect;
        self.count += 1;
    }

    // ------------------------=
    // FUNC: regions
    // DESC: Exposes only initialized damage rectangles to the platform presenter.
    // ------------------=
    pub fn regions(&self) -> &[Rect] {
        &self.regions[..self.count as usize]
    }

    // ------------------------=
    // FUNC: collapsed
    // DESC: Reports that excessive small changes were collapsed into one safe region.
    // ------------------=
    pub const fn collapsed(&self) -> bool {
        self.full_redraw
    }
}
