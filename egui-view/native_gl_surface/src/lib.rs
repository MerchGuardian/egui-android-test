#[deny(unsafe_op_in_unsafe_fn)]
use std::ffi::CString;
use std::{ffi::c_void, sync::OnceLock};

use android_logger::Config;
use jni::{
    objects::JClass,
    sys::{jint, jlong, JNIEnv},
};
use log::{info, trace, LevelFilter};

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLRenderer_onDrawFrame0(
    _: JNIEnv,
    _: JClass,
    _native_surface: jlong,
) {
    trace!("onDrawFrame0 called");

    let ctx = get_glow_context();

    unsafe {
        use glow::*;
        ctx.clear(COLOR_BUFFER_BIT);
        ctx.clear_color(1.0, 0.0, 1.0, 1.0);
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
    _native_surface: jlong,
    width: jint,
    height: jint,
) {
    info!("onSurfaceChanged0 called: width: {width}, height: {height}");

    let ctx = get_glow_context();

    unsafe {
        use glow::*;
        ctx.viewport(0, 0, width, height);
    }
    info!("Set gl viewport");
}

struct NativeSurface {}

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLSurfaceView_createNativeSurface0(
    _: JNIEnv,
    _: JClass,
) -> jlong {
    let native_surface = NativeSurface {};
    let ptr: *mut NativeSurface = Box::into_raw(Box::new(native_surface));
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
    let _handle = unsafe { Box::from_raw(ptr) };
}

static GL_FUNCTIONS: std::sync::OnceLock<glow::Context> = OnceLock::new();

pub fn get_glow_context() -> &'static glow::Context {
    GL_FUNCTIONS.get_or_init(|| {
        info!("Creating glow wrapper");
        fn load_gl_func(symbol_name: &str) -> *const c_void {
            let c_str = CString::new(symbol_name).unwrap();
            // SAFETY: function provided by android
            unsafe { eglGetProcAddress(c_str.as_ptr().cast()) }
        }
        let glow_context = unsafe { glow::Context::from_loader_function(load_gl_func) };
        glow_context
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
