#[deny(unsafe_op_in_unsafe_fn)]
use std::ffi::CString;
use std::{
    ffi::c_void,
    sync::{Arc, OnceLock},
    time::Instant,
};

use android_logger::Config;
use egui::{
    ahash::HashMapExt,
    emath::{RectTransform, Rot2},
    vec2, Color32, Frame, Pos2, Rect, Sense, Stroke, Vec2,
};
use jni::{
    objects::JClass,
    sys::{jint, jlong, JNIEnv},
};
use log::{info, trace, LevelFilter};

struct AppState {
    clear_color: [f32; 3],
    rotation: f32,
    translation: Vec2,
    zoom: f32,
    last_touch_time: f64,
    name: String,
    age: u32,
}

impl AppState {
    fn new() -> Self {
        Self {
            clear_color: [1.0, 0.0, 1.0],
            rotation: 0.,
            translation: Vec2::ZERO,
            zoom: 1.,
            last_touch_time: 0.0,
            name: String::from("Bob"),
            age: 69,
        }
    }

    fn draw(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                self.age += 1;
            }
            ui.label(format!("Hello '{}', age {}", self.name, self.age));

            ui.color_edit_button_rgb(&mut self.clear_color);

            // ui.image(egui::include_image!(
            //     "../../../crates/egui/assets/ferris.png"
            // ));
            Frame::canvas(ui.style()).show(ui, |ui| {
                let num_touches = ui.input(|i| i.multi_touch().map_or(0, |mt| mt.num_touches));
                ui.label(format!("Current touches: {num_touches}"));

                let color = if ui.visuals().dark_mode {
                    Color32::WHITE
                } else {
                    Color32::BLACK
                };

                // Note that we use `Sense::drag()` although we do not use any pointer events. With
                // the current implementation, the fact that a touch event of two or more fingers is
                // recognized, does not mean that the pointer events are suppressed, which are always
                // generated for the first finger. Therefore, if we do not explicitly consume pointer
                // events, the window will move around, not only when dragged with a single finger, but
                // also when a two-finger touch is active. I guess this problem can only be cleanly
                // solved when the synthetic pointer events are created by egui, and not by the
                // backend.

                // set up the drawing canvas with normalized coordinates:
                let (response, painter) =
                    ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

                // normalize painter coordinates to ±1 units in each direction with [0,0] in the center:
                let painter_proportions = response.rect.square_proportions();
                let to_screen = RectTransform::from_to(
                    Rect::from_min_size(Pos2::ZERO - painter_proportions, 2. * painter_proportions),
                    response.rect,
                );

                // check for touch input (or the lack thereof) and update zoom and scale factors, plus
                // color and width:
                let mut stroke_width = 1.;
                if let Some(multi_touch) = ui.ctx().multi_touch() {
                    // This adjusts the current zoom factor and rotation angle according to the dynamic
                    // change (for the current frame) of the touch gesture:
                    self.zoom *= multi_touch.zoom_delta;
                    self.rotation += multi_touch.rotation_delta;
                    // the translation we get from `multi_touch` needs to be scaled down to the
                    // normalized coordinates we use as the basis for painting:
                    self.translation += to_screen.inverse().scale() * multi_touch.translation_delta;
                    // touch pressure will make the arrow thicker (not all touch devices support this):
                    stroke_width += 10. * multi_touch.force;

                    self.last_touch_time = ui.input(|i| i.time);
                } else {
                    self.slowly_reset(ui);
                }
                let zoom_and_rotate = self.zoom * Rot2::from_angle(self.rotation);
                let arrow_start_offset = self.translation + zoom_and_rotate * vec2(-0.5, 0.5);

                // Paints an arrow pointing from bottom-left (-0.5, 0.5) to top-right (0.5, -0.5), but
                // scaled, rotated, and translated according to the current touch gesture:
                let arrow_start = Pos2::ZERO + arrow_start_offset;
                let arrow_direction = zoom_and_rotate * vec2(1., -1.);
                painter.arrow(
                    to_screen * arrow_start,
                    to_screen.scale() * arrow_direction,
                    Stroke::new(stroke_width, color),
                );
            });
        });
    }

    fn slowly_reset(&mut self, ui: &egui::Ui) {
        // This has nothing to do with the touch gesture. It just smoothly brings the
        // painted arrow back into its original position, for a nice visual effect:

        let time_since_last_touch = (ui.input(|i| i.time) - self.last_touch_time) as f32;

        let delay = 0.5;
        if time_since_last_touch < delay {
            ui.ctx().request_repaint();
        } else {
            // seconds after which half the amount of zoom/rotation will be reverted:
            let half_life =
                egui::remap_clamp(time_since_last_touch, delay..=1.0, 1.0..=0.0).powf(4.0);

            if half_life <= 1e-3 {
                self.zoom = 1.0;
                self.rotation = 0.0;
                self.translation = Vec2::ZERO;
            } else {
                let dt = ui.input(|i| i.unstable_dt);
                let half_life_factor = (-(2_f32.ln()) / half_life * dt).exp();
                self.zoom = 1. + ((self.zoom - 1.) * half_life_factor);
                self.rotation *= half_life_factor;
                self.translation *= half_life_factor;
                ui.ctx().request_repaint();
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLRenderer_onDrawFrame0(
    _: JNIEnv,
    _: JClass,
    native_surface: jlong,
) {
    trace!("onDrawFrame0 called");
    let ptr = native_surface as usize as *mut NativeSurface;
    let wrapper = unsafe { &mut *ptr };
    if wrapper.inner.is_none() {
        info!("Creating lazy RustSurface");
        wrapper.inner = Some(RustSurface::new());
    }
    let surface = wrapper.inner.as_mut().unwrap();

    // The size of the surface, in points
    let surface_size = wrapper
        .raw_surface_size
        .map(|s| s / wrapper.native_pixels_per_point)
        .unwrap_or_default();

    let mut viewports = egui::viewport::ViewportIdMap::new();
    viewports.insert(
        egui::ViewportId::ROOT,
        egui::ViewportInfo {
            native_pixels_per_point: Some(wrapper.native_pixels_per_point),
            monitor_size: Some(surface_size),
            focused: Some(true),
            ..Default::default()
        },
    );

    let raw_input = egui::RawInput {
        viewport_id: egui::ViewportId::ROOT,
        viewports,
        // TODO: Obtain value
        max_texture_side: None,
        screen_rect: Some(Rect::from_min_size(Default::default(), surface_size)),
        time: Some(surface.timer.elapsed().as_secs_f64()),
        ..Default::default()
    };

    let egui::FullOutput {
        platform_output: _,
        mut textures_delta,
        shapes,
        pixels_per_point,
        viewport_output,
    } = surface
        .egui_ctx
        .run(raw_input, |ctx| surface.app_state.draw(ctx));

    assert!(
        viewport_output.len() <= 1,
        "Multiple viewports not yet supported by EguiGlow"
    );

    for (_, egui::ViewportOutput { commands: _, .. }) in viewport_output {
        // TODO: handle
    }

    // self.egui_winit.handle_platform_output(window, platform_output);

    let window_size = wrapper.raw_surface_size.unwrap_or_default();
    let window_size = [window_size.x as u32, window_size.y as u32];
    let cc = surface.app_state.clear_color;
    let clear_color = [cc[0], cc[1], cc[2], 1.0];
    surface.painter.clear(window_size, clear_color);

    for (id, image_delta) in textures_delta.set {
        info!("Setting texture: {id:?}");
        surface.painter.set_texture(id, &image_delta);
    }

    let clipped_primitives = surface.egui_ctx.tessellate(shapes, pixels_per_point);
    surface
        .painter
        .paint_primitives(window_size, pixels_per_point, &clipped_primitives);

    for id in textures_delta.free.drain(..) {
        surface.painter.free_texture(id);
    }
}

struct RustSurface {
    egui_ctx: egui::Context,
    painter: egui_glow::Painter,
    timer: Instant,
    app_state: AppState,
}

impl RustSurface {
    pub fn new() -> Self {
        let gl = get_glow_context();

        Self {
            egui_ctx: egui::Context::default(),
            timer: Instant::now(),
            painter: egui_glow::Painter::new(Arc::clone(gl), "", None, false)
                .expect("Failed to create glow painter"),
            app_state: AppState::new(),
        }
    }
}

#[derive(Default)]
struct NativeSurface {
    inner: Option<RustSurface>,
    raw_surface_size: Option<egui::Vec2>,
    native_pixels_per_point: f32,
}

impl NativeSurface {
    pub fn new() -> Self {
        Self {
            inner: None,
            raw_surface_size: None,
            native_pixels_per_point: 3.0,
        }
    }
}

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLRenderer_onSurfaceCreated0(
    _: JNIEnv,
    _: JClass,
    _native_surface: jlong,
) {
    info!("onSurfaceCreated0 called");
}

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLRenderer_onSurfaceChanged0(
    _: JNIEnv,
    _: JClass,
    native_surface: jlong,
    width: jint,
    height: jint,
) {
    info!("onSurfaceChanged0 called: width: {width}, height: {height}");
    let ptr = native_surface as usize as *mut NativeSurface;
    let surface = unsafe { &mut *ptr };
    surface.raw_surface_size = Some(egui::Vec2::new(width as f32, height as f32));
    info!("onSurfaceChanged0 done");
}

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLSurfaceView_createNativeSurface0(
    _: JNIEnv,
    _: JClass,
) -> jlong {
    info!("createNativeSurface called");

    let ptr: *mut NativeSurface = Box::into_raw(Box::new(NativeSurface::new()));
    info!("Allocated native surface {ptr:?}");
    ptr as usize as jlong
    //
}

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLSurfaceView_destroyNativeSurface0(
    _: JNIEnv,
    _: JClass,
    native_surface: jlong,
) {
    let ptr = native_surface as usize as *mut NativeSurface;
    info!("Destroying native surface {ptr:?}");
    let mut surface = unsafe { Box::from_raw(ptr) };

    if let Some(inner) = surface.inner.as_mut() {
        inner.painter.destroy();
    }
}

static GL_FUNCTIONS: std::sync::OnceLock<Arc<glow::Context>> = OnceLock::new();

pub fn get_glow_context() -> &'static Arc<glow::Context> {
    GL_FUNCTIONS.get_or_init(|| {
        info!("Creating glow wrapper");
        fn load_gl_func(symbol_name: &str) -> *const c_void {
            let c_str = CString::new(symbol_name).unwrap();
            // SAFETY: function provided by android
            unsafe { eglGetProcAddress(c_str.as_ptr().cast()) }
        }
        let glow_context = unsafe { glow::Context::from_loader_function(load_gl_func) };
        Arc::new(glow_context)
    })
}

extern "C" {
    fn eglGetProcAddress(procname: *const i8) -> *const c_void;
}

#[no_mangle]
pub extern "C" fn JNI_OnLoad(vm: jni::JavaVM, res: *mut std::os::raw::c_void) -> jni::sys::jint {
    android_logger::init_once(
        Config::default()
            .with_tag("com.foxhunter.egui_view")
            .format(|f, record| write!(f, "jni: {}", record.args()))
            .with_max_level(LevelFilter::Debug),
    );

    std::panic::set_hook(Box::new(|info| {
        let (file, line) = {
            if let Some(location) = info.location() {
                (location.file(), location.line())
            } else {
                ("<unknown>", 0)
            }
        };

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };

        log::error!("foxhunterjni PANICKED: {msg}, at {file}:{line}");
    }));

    let vm = vm.get_java_vm_pointer() as *mut std::ffi::c_void;
    info!("Java VM pointer: {vm:?}");
    unsafe {
        ndk_context::initialize_android_context(vm, res);
    }
    info!("Initialized ndk context!");
    info!("Returning from JNI_OnLoad");

    jni::JNIVersion::V6.into()
}
