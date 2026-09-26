//! Native, bounded CPU render surface. WebRender runs through SWGL's Gleam API.
use super::{Error, Framebuffer, RenderingContext};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use dpi::PhysicalSize;
use gleam::gl::{self, Gl};
use image::RgbaImage;
use webrender_api::units::DeviceIntRect;

pub struct SoftwareRenderingContext {
    context: swgl::Context,
    size: Cell<PhysicalSize<u32>>,
    queries: Arc<glow::Context>,
}

impl SoftwareRenderingContext {
    // ------------------------=
    // FUNC: new
    // DESC: Allocates an actual CPU framebuffer without a display server or GPU connection.
    // ------------------=
    pub fn new(size: PhysicalSize<u32>) -> Result<Self, Error> {
        if !Self::valid_size(size) { return Err(Error::Failed); }
        let context = swgl::Context::create();
        context.make_current();
        context.init_default_framebuffer(0, 0, size.width as i32, size.height as i32,
                                         0, std::ptr::null_mut());
        let queries = query_context();
        Ok(Self { context, size: Cell::new(size), queries })
    }

    // ------------------------=
    // FUNC: valid_size
    // DESC: Bounds viewport backing to sixteen MiB of color pixels before allocation.
    // ------------------=
    fn valid_size(size: PhysicalSize<u32>) -> bool {
        size.width > 0 && size.height > 0 && size.width <= 2048 && size.height <= 2048
    }
}

impl RenderingContext for SoftwareRenderingContext {
    // ------------------------=
    // FUNC: prepare_for_rendering
    // DESC: Selects this CPU context and its persistent default framebuffer.
    // ------------------=
    fn prepare_for_rendering(&self) {
        self.context.make_current();
        self.context.bind_framebuffer(gl::FRAMEBUFFER, 0);
    }
    // ------------------------=
    // FUNC: read_to_image
    // DESC: Reads real raster pixels only for an in-bounds rectangle.
    // ------------------=
    fn read_to_image(&self, rectangle: DeviceIntRect) -> Option<RgbaImage> {
        let size = self.size.get();
        if rectangle.min.x < 0 || rectangle.min.y < 0 || rectangle.is_empty() ||
           rectangle.max.x > size.width as i32 || rectangle.max.y > size.height as i32 {
            return None;
        }
        self.prepare_for_rendering();
        Framebuffer::read_framebuffer_to_image(&self.gleam_gl_api(), 0, rectangle)
    }
    // ------------------------=
    // FUNC: size
    // DESC: Returns the dimensions of the currently allocated viewport.
    // ------------------=
    fn size(&self) -> PhysicalSize<u32> { self.size.get() }
    // ------------------------=
    // FUNC: resize
    // DESC: Reallocates only on a valid size change, retaining the previous surface on invalid input.
    // ------------------=
    fn resize(&self, size: PhysicalSize<u32>) {
        if !Self::valid_size(size) || self.size.get() == size { return; }
        self.context.make_current();
        self.context.init_default_framebuffer(0, 0, size.width as i32, size.height as i32,
                                         0, std::ptr::null_mut());
        self.size.set(size);
    }
    // ------------------------=
    // FUNC: present
    // DESC: Resolves delayed CPU clears; InfinityUI owns final window presentation.
    // ------------------=
    fn present(&self) {
        self.context.make_current();
        self.context.resolve_framebuffer(0);
    }
    // ------------------------=
    // FUNC: make_current
    // DESC: Selects this native software context on the governed renderer worker.
    // ------------------=
    fn make_current(&self) -> Result<(), Error> { self.context.make_current(); Ok(()) }
    // ------------------------=
    // FUNC: gleam_gl_api
    // DESC: Exposes upstream SWGL's actual raster operations to WebRender.
    // ------------------=
    fn gleam_gl_api(&self) -> Rc<dyn Gl> { Rc::new(self.context) }
    // ------------------------=
    // FUNC: glow_gl_api
    // DESC: Exposes real software capability queries; GPU compositing functions remain unavailable.
    // ------------------=
    fn glow_gl_api(&self) -> Arc<glow::Context> { self.queries.clone() }
}

impl Drop for SoftwareRenderingContext {
    // ------------------------=
    // FUNC: drop
    // DESC: Releases SWGL-owned color, depth, texture and shader resources.
    // ------------------=
    fn drop(&mut self) { self.context.destroy(); }
}

// ------------------------=
// FUNC: query_context
// DESC: Binds actual SWGL query symbols, never a fabricated OpenGL driver or successful GPU stub.
// ------------------=
#[expect(unsafe_code)]
fn query_context() -> Arc<glow::Context> {
    unsafe extern "C" {
        fn GetString(name: u32) -> *const u8;
        fn GetStringi(name: u32, index: u32) -> *const u8;
        fn GetIntegerv(name: u32, output: *mut i32);
    }
    // SAFETY: SWGL is current. These symbols have the GL query ABI and static
    // lifetime. Unsupported entry points remain null rather than fake success.
    Arc::new(unsafe { glow::Context::from_loader_function(|name| match name {
        "glGetString" => GetString as *const () as *const _,
        "glGetStringi" => GetStringi as *const () as *const _,
        "glGetIntegerv" => GetIntegerv as *const () as *const _,
        _ => std::ptr::null(),
    }) })
}
