//! Software retained-surface composition into a caller-owned back buffer.

use super::geometry::{Rect, Size};
use super::scene::{DamageRecord, DamageTracker};
use super::surface::{PixelFormat, SurfaceDescriptor};
use super::window::ZOrderClass;

#[derive(Clone, Copy)]
pub struct SurfaceFrame<'a> {
    pub descriptor: SurfaceDescriptor,
    pub pixels: &'a [u32],
    pub bounds: Rect,
    pub opacity: u8,
    pub z_class: ZOrderClass,
    pub visible: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompositorError {
    InvalidDisplay,
    BackBufferTooSmall,
    FrontBufferTooSmall,
    SurfaceBufferTooSmall,
    PrivilegedZOrderDenied,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct CompositorMetrics {
    pub composed_frames: u64,
    pub presented_frames: u64,
    pub composed_pixels: u64,
    pub presented_pixels: u64,
    pub rejected_layers: u32,
    pub damage_collapses: u32,
    pub deferred_regions: u32,
}

pub struct SoftwareCompositor {
    display: Size,
    stride_pixels: usize,
    background: u32,
    frame_ready: bool,
    metrics: CompositorMetrics,
}

impl SoftwareCompositor {
    // ------------------------=
    // FUNC: new
    // DESC: Creates a software compositor that stages complete damaged regions before presentation.
    // ------------------=
    pub const fn new(display: Size, stride_pixels: usize, background: u32) -> Self {
        Self {
            display,
            stride_pixels,
            background,
            frame_ready: false,
            metrics: CompositorMetrics {
                composed_frames: 0,
                presented_frames: 0,
                composed_pixels: 0,
                presented_pixels: 0,
                rejected_layers: 0,
                damage_collapses: 0,
                deferred_regions: 0,
            },
        }
    }

    // ------------------------=
    // FUNC: required_pixels
    // DESC: Calculates the caller-owned buffer length required by this display geometry.
    // ------------------=
    pub fn required_pixels(&self) -> Result<usize, CompositorError> {
        if self.display.width == 0
            || self.display.height == 0
            || self.stride_pixels < self.display.width as usize
        {
            return Err(CompositorError::InvalidDisplay);
        }
        self.stride_pixels
            .checked_mul(self.display.height as usize)
            .ok_or(CompositorError::InvalidDisplay)
    }

    // ------------------------=
    // FUNC: compose
    // DESC: Composes semantic damage into a back buffer in policy-defined z-order without exposing it early.
    // ------------------=
    pub fn compose(
        &mut self,
        back_buffer: &mut [u32],
        layers: &[SurfaceFrame<'_>],
        damage: &DamageTracker,
    ) -> Result<(), CompositorError> {
        let required = self.required_pixels()?;
        if back_buffer.len() < required {
            return Err(CompositorError::BackBufferTooSmall);
        }
        let display_rect = Rect {
            x: 0,
            y: 0,
            width: self.display.width,
            height: self.display.height,
        };
        for region in damage.regions() {
            let clipped = region.intersection(display_rect);
            self.fill_region(back_buffer, clipped, self.background);
            self.metrics.composed_pixels =
                self.metrics.composed_pixels.saturating_add(area(clipped));
            for z in 0..=ZOrderClass::Cursor as u8 {
                for layer in layers {
                    if !layer.visible || layer.z_class as u8 != z {
                        continue;
                    }
                    if !layer_authorized(layer) {
                        self.metrics.rejected_layers =
                            self.metrics.rejected_layers.saturating_add(1);
                        return Err(CompositorError::PrivilegedZOrderDenied);
                    }
                    let source_required = layer.descriptor.stride_pixels as usize
                        * layer.descriptor.size.height as usize;
                    if layer.pixels.len() < source_required {
                        return Err(CompositorError::SurfaceBufferTooSmall);
                    }
                    self.blend_layer(back_buffer, layer, clipped);
                }
            }
        }
        self.metrics.composed_frames = self.metrics.composed_frames.wrapping_add(1);
        if damage.collapsed() {
            self.metrics.damage_collapses = self.metrics.damage_collapses.saturating_add(1);
        }
        self.frame_ready = true;
        Ok(())
    }

    // ------------------------=
    // FUNC: present
    // DESC: Copies only fully composed damaged regions from the back buffer to the visible front buffer.
    // ------------------=
    pub fn present(
        &mut self,
        front_buffer: &mut [u32],
        back_buffer: &[u32],
        damage: &DamageTracker,
    ) -> Result<(), CompositorError> {
        let mut deferred = [DamageRecord {
            rect: Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            class: super::scene::DamageClass::Content,
            source_id: 0,
            priority: 0,
        }; super::scene::MAX_DAMAGE_REGIONS];
        self.present_prioritized(front_buffer, back_buffer, damage, u64::MAX, &mut deferred)?;
        Ok(())
    }

    // ------------------------=
    // FUNC: present_prioritized
    // DESC: Presents high-priority semantic damage first and returns lower-priority regions for a later frame.
    // ------------------=
    pub fn present_prioritized(
        &mut self,
        front_buffer: &mut [u32],
        back_buffer: &[u32],
        damage: &DamageTracker,
        pixel_budget: u64,
        deferred: &mut [DamageRecord],
    ) -> Result<usize, CompositorError> {
        let required = self.required_pixels()?;
        if front_buffer.len() < required {
            return Err(CompositorError::FrontBufferTooSmall);
        }
        if back_buffer.len() < required {
            return Err(CompositorError::BackBufferTooSmall);
        }
        if !self.frame_ready {
            return Ok(0);
        }
        let display_rect = Rect {
            x: 0,
            y: 0,
            width: self.display.width,
            height: self.display.height,
        };
        let mut consumed_pixels = 0u64;
        let mut deferred_count = 0usize;
        for priority in (0u16..=255).rev() {
            for record in damage.records() {
                if record.priority as u16 != priority {
                    continue;
                }
                let clipped = record.rect.intersection(display_rect);
                let pixels = area(clipped);
                let protected = record.priority >= 240;
                if !protected && consumed_pixels.saturating_add(pixels) > pixel_budget {
                    if deferred_count < deferred.len() {
                        deferred[deferred_count] = *record;
                        deferred_count += 1;
                    }
                    self.metrics.deferred_regions = self.metrics.deferred_regions.saturating_add(1);
                    continue;
                }
                for y in clipped.y.max(0) as usize..clipped.bottom().max(0) as usize {
                    let left = y * self.stride_pixels + clipped.x.max(0) as usize;
                    let right = left + clipped.width as usize;
                    front_buffer[left..right].copy_from_slice(&back_buffer[left..right]);
                }
                consumed_pixels = consumed_pixels.saturating_add(pixels);
                self.metrics.presented_pixels =
                    self.metrics.presented_pixels.saturating_add(pixels);
            }
        }
        self.metrics.presented_frames = self.metrics.presented_frames.wrapping_add(1);
        self.frame_ready = deferred_count != 0;
        Ok(deferred_count)
    }

    // ------------------------=
    // FUNC: metrics
    // DESC: Returns structured compositor counters for diagnostics and behavioral tests.
    // ------------------=
    pub const fn metrics(&self) -> CompositorMetrics {
        self.metrics
    }

    // ------------------------=
    // FUNC: fill_region
    // DESC: Clears one clipped back-buffer region before retained layers are blended.
    // ------------------=
    fn fill_region(&self, back_buffer: &mut [u32], region: Rect, color: u32) {
        for y in region.y.max(0) as usize..region.bottom().max(0) as usize {
            let left = y * self.stride_pixels + region.x.max(0) as usize;
            let right = left + region.width as usize;
            back_buffer[left..right].fill(color);
        }
    }

    // ------------------------=
    // FUNC: blend_layer
    // DESC: Blends the intersecting portion of one retained surface into the staged frame.
    // ------------------=
    fn blend_layer(&self, back_buffer: &mut [u32], layer: &SurfaceFrame<'_>, damage: Rect) {
        let target = layer.bounds.intersection(damage);
        if target.width == 0 || target.height == 0 {
            return;
        }
        for y in target.y.max(0) as usize..target.bottom().max(0) as usize {
            for x in target.x.max(0) as usize..target.right().max(0) as usize {
                let source_x = x.saturating_sub(layer.bounds.x.max(0) as usize);
                let source_y = y.saturating_sub(layer.bounds.y.max(0) as usize);
                if source_x >= layer.descriptor.size.width as usize
                    || source_y >= layer.descriptor.size.height as usize
                {
                    continue;
                }
                let source_index = source_y * layer.descriptor.stride_pixels as usize + source_x;
                let target_index = y * self.stride_pixels + x;
                let source = layer.pixels[source_index];
                let alpha = match layer.descriptor.format {
                    PixelFormat::Xrgb8888 => layer.opacity,
                    PixelFormat::Argb8888 => {
                        (((source >> 24) as u16 * layer.opacity as u16) / 255) as u8
                    }
                };
                back_buffer[target_index] = blend(source, back_buffer[target_index], alpha);
            }
        }
    }
}

// ------------------------=
// FUNC: layer_authorized
// DESC: Enforces that only trusted or cursor-class surfaces occupy reserved compositor z-order classes.
// ------------------=
fn layer_authorized(layer: &SurfaceFrame<'_>) -> bool {
    use super::surface::SurfaceSecurityClass;
    match layer.z_class {
        ZOrderClass::Trusted => layer.descriptor.security_class == SurfaceSecurityClass::Trusted,
        ZOrderClass::Cursor => layer.descriptor.security_class == SurfaceSecurityClass::Cursor,
        _ => layer.descriptor.security_class <= SurfaceSecurityClass::System,
    }
}

// ------------------------=
// FUNC: blend
// DESC: Alpha blends one packed RGB pixel using integer arithmetic.
// ------------------=
fn blend(source: u32, destination: u32, alpha: u8) -> u32 {
    if alpha == 255 {
        return source | 0xff00_0000;
    }
    if alpha == 0 {
        return destination;
    }
    let inverse = 255u32.saturating_sub(alpha as u32);
    let a = alpha as u32;
    let red = (((source >> 16) & 0xff) * a + ((destination >> 16) & 0xff) * inverse) / 255;
    let green = (((source >> 8) & 0xff) * a + ((destination >> 8) & 0xff) * inverse) / 255;
    let blue = ((source & 0xff) * a + (destination & 0xff) * inverse) / 255;
    0xff00_0000 | (red << 16) | (green << 8) | blue
}

// ------------------------=
// FUNC: area
// DESC: Calculates a rectangle's pixel area for structured diagnostics.
// ------------------=
fn area(rect: Rect) -> u64 {
    u64::from(rect.width).saturating_mul(u64::from(rect.height))
}
