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
    DeferredBufferTooSmall,
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
        self.compose_layers(back_buffer, layers, damage, false)
    }

    // ------------------------=
    // FUNC: compose_transformed
    // DESC: Scales persistent surfaces into animated destination bounds without rerendering applications.
    // ------------------=
    pub fn compose_transformed(
        &mut self,
        back_buffer: &mut [u32],
        layers: &[SurfaceFrame<'_>],
        damage: &DamageTracker,
    ) -> Result<(), CompositorError> {
        self.compose_layers(back_buffer, layers, damage, true)
    }

    // ------------------------=
    // FUNC: compose_layers
    // DESC: Validates and stages only damaged regions while preserving identical authorization for animated layers.
    // ------------------=
    fn compose_layers(
        &mut self,
        back_buffer: &mut [u32],
        layers: &[SurfaceFrame<'_>],
        damage: &DamageTracker,
        transformed: bool,
    ) -> Result<(), CompositorError> {
        self.frame_ready = false;
        let required = self.required_pixels()?;
        if back_buffer.len() < required {
            return Err(CompositorError::BackBufferTooSmall);
        }
        // Validate the whole transaction before touching staged pixels.
        for layer in layers.iter().filter(|layer| layer.visible) {
            if !layer_authorized(layer) {
                self.metrics.rejected_layers = self.metrics.rejected_layers.saturating_add(1);
                return Err(CompositorError::PrivilegedZOrderDenied);
            }
            let descriptor = layer.descriptor;
            let source_required = (descriptor.stride_pixels as usize)
                .checked_mul(descriptor.size.height as usize)
                .ok_or(CompositorError::SurfaceBufferTooSmall)?;
            if descriptor.stride_pixels < descriptor.size.width
                || layer.pixels.len() < source_required
            {
                return Err(CompositorError::SurfaceBufferTooSmall);
            }
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
                    if transformed
                        && (layer.bounds.width != layer.descriptor.size.width
                            || layer.bounds.height != layer.descriptor.size.height)
                    {
                        self.blend_transformed(back_buffer, layer, clipped);
                    } else {
                        self.blend_layer(back_buffer, layer, clipped);
                    }
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
        // Only short caller buffers need preflight. Never lose damage or exceed the
        // responsiveness budget to compensate for an undersized output buffer.
        if deferred.len() < damage.records().len() {
            let mut planned_pixels = 0u64;
            let mut pending = 0;
            for priority in (0u16..=255).rev() {
                for record in damage
                    .records()
                    .iter()
                    .filter(|record| record.priority as u16 == priority)
                {
                    let pixels = area(record.rect.intersection(display_rect));
                    if record.priority < 240 && planned_pixels.saturating_add(pixels) > pixel_budget
                    {
                        pending += 1;
                    } else {
                        planned_pixels = planned_pixels.saturating_add(pixels);
                    }
                }
            }
            if pending > deferred.len() {
                return Err(CompositorError::DeferredBufferTooSmall);
            }
        }
        for priority in (0u16..=255).rev() {
            for record in damage.records() {
                if record.priority as u16 != priority {
                    continue;
                }
                let clipped = record.rect.intersection(display_rect);
                let pixels = area(clipped);
                let protected = record.priority >= 240;
                if !protected && consumed_pixels.saturating_add(pixels) > pixel_budget {
                    deferred[deferred_count] = *record;
                    deferred_count += 1;
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
        if consumed_pixels != 0 {
            self.metrics.presented_frames = self.metrics.presented_frames.wrapping_add(1);
        }
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
            let source_y = (y as i64 - i64::from(layer.bounds.y)) as usize;
            if source_y >= layer.descriptor.size.height as usize {
                continue;
            }
            // Opaque rows can be copied directly from the persistent app surface.
            if layer.opacity == 255 && layer.descriptor.format == PixelFormat::Xrgb8888 {
                let left = target.x.max(0) as usize;
                let source_x = (left as i64 - i64::from(layer.bounds.x)) as usize;
                let width = (target.width as usize)
                    .min((layer.descriptor.size.width as usize).saturating_sub(source_x));
                if width != 0 {
                    let source = source_y * layer.descriptor.stride_pixels as usize + source_x;
                    let destination = y * self.stride_pixels + left;
                    for (out, pixel) in back_buffer[destination..destination + width]
                        .iter_mut()
                        .zip(&layer.pixels[source..source + width])
                    {
                        *out = *pixel | 0xff00_0000;
                    }
                }
                continue;
            }
            for x in target.x.max(0) as usize..target.right().max(0) as usize {
                let source_x = (x as i64 - i64::from(layer.bounds.x)) as usize;
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

    // ------------------------=
    // FUNC: blend_transformed
    // DESC: Bilinearly samples retained pixels with premultiplied-alpha interpolation and bounded damage.
    // ------------------=
    fn blend_transformed(&self, back: &mut [u32], layer: &SurfaceFrame<'_>, damage: Rect) {
        let target = layer.bounds.intersection(damage);
        let size = layer.descriptor.size;
        if size.width == 0
            || size.height == 0
            || layer.bounds.width == 0
            || layer.bounds.height == 0
        {
            return;
        }
        for y in target.y.max(0)..target.bottom().max(0) {
            let sy = (((i64::from(y) - i64::from(layer.bounds.y)) * 2 + 1)
                * i64::from(size.height)
                * 128
                / i64::from(layer.bounds.height)
                - 128)
                .clamp(0, i64::from(size.height - 1) * 256);
            for x in target.x.max(0)..target.right().max(0) {
                let sx = (((i64::from(x) - i64::from(layer.bounds.x)) * 2 + 1)
                    * i64::from(size.width)
                    * 128
                    / i64::from(layer.bounds.width)
                    - 128)
                    .clamp(0, i64::from(size.width - 1) * 256);
                let x0 = (sx / 256) as usize;
                let y0 = (sy / 256) as usize;
                let x1 = (x0 + 1).min(size.width as usize - 1);
                let y1 = (y0 + 1).min(size.height as usize - 1);
                let fx = (sx % 256) as u64;
                let fy = (sy % 256) as u64;
                let mut alpha = 0u64;
                let mut channels = [0u64; 3];
                for (px, py, weight) in [
                    (x0, y0, (256 - fx) * (256 - fy)),
                    (x1, y0, fx * (256 - fy)),
                    (x0, y1, (256 - fx) * fy),
                    (x1, y1, fx * fy),
                ] {
                    let pixel = layer.pixels[py * layer.descriptor.stride_pixels as usize + px];
                    let a = if layer.descriptor.format == PixelFormat::Xrgb8888 {
                        255
                    } else {
                        u64::from(pixel >> 24)
                    };
                    alpha += a * weight;
                    for (index, channel) in channels.iter_mut().enumerate() {
                        *channel += u64::from((pixel >> (index * 8)) & 255) * a * weight;
                    }
                }
                if alpha != 0 {
                    let mut pixel = 0u32;
                    for (index, channel) in channels.iter().enumerate() {
                        pixel |= ((channel / alpha) as u32) << (index * 8);
                    }
                    let opacity = (alpha * u64::from(layer.opacity) / (65536 * 255)) as u8;
                    let destination = y as usize * self.stride_pixels + x as usize;
                    back[destination] = blend(pixel, back[destination], opacity);
                }
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
