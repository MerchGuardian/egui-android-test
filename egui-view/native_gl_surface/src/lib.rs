use std::ffi::c_void;
#[deny(unsafe_op_in_unsafe_fn)]
use std::ffi::CString;

use android_logger::Config;
use jni::{
    objects::JClass,
    sys::{jint, jlong, JNIEnv},
};
use log::{info, LevelFilter};

macro_rules! get_wrapper {
    ($handle:ident) => {{
        assert!($handle != 0);
        let ptr = $handle as usize as *const GLContextWrapper;
        let wrapper = unsafe { &*ptr };
        wrapper
    }};
}

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLRenderer_onDrawFrame0(
    _: JNIEnv,
    _: JClass,
    context_handle: jlong,
) {
    info!("onDrawFrame0 called");
    let wrapper = get_wrapper!(context_handle);

    wrapper.with_context(|ctx| unsafe {
        info!("Rendering frame!");

        use glow::HasContext;
        ctx.clear(glow::COLOR_BUFFER_BIT);
        ctx.clear_color(1.0, 0.0, 1.0, 1.0);
    });
}

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLRenderer_onSurfaceCreated0(
    _: JNIEnv,
    _: JClass,
    context_handle: jlong,
) {
    info!("onSurfaceCreated0 called");

    let wrapper = get_wrapper!(context_handle);
    //  Called on different thread than rendering thread
    // wrapper.with_context(|_ctx| {
    //     info!("Surface created!");
    // });
}

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLRenderer_onSurfaceChanged0(
    _: JNIEnv,
    _: JClass,
    context_handle: jlong,
    width: jint,
    height: jint,
) {
    info!("onSurfaceChanged0 called: context_handle: {context_handle:X?}, width: {width}, height: {height}");

    let wrapper = get_wrapper!(context_handle);
    wrapper.with_context(|ctx| unsafe {
        use glow::HasContext;
        ctx.viewport(0, 0, width, height);
        info!("Set gl viewport");
    });
}

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLRenderer_createNativeContext0(
    _: JNIEnv,
    _: JClass,
) -> jlong {
    info!("createNativeContext0 called");
    let context = GLContextWrapper::new();
    let ptr: *mut GLContextWrapper = Box::into_raw(Box::new(context));

    info!("Created native context {ptr:?}");
    ptr as usize as jlong
}

#[no_mangle]
pub extern "C" fn Java_com_foxhunter_egui_1view_ui_NativeGLRenderer_destroyNativeContext0(
    _: JNIEnv,
    _: JClass,
    handle: jlong,
) {
    info!("destroyNativeContext0 called {handle:X?}");

    if handle == 0 {
        return;
    }
    let ptr = handle as usize as *mut GLContextWrapper;
    info!("Destroying native context {ptr:?}");
    let _context = unsafe { Box::from_raw(ptr) };
}

struct GLContext {
    glow_context: glow::Context,
    /// The thread on which the GL context is current.
    current_thread: std::thread::ThreadId,
}

struct GLContextWrapper {
    inner: parking_lot::Mutex<Option<GLContext>>,
}

impl GLContextWrapper {
    pub fn new() -> Self {
        Self {
            inner: parking_lot::Mutex::new(None),
        }
    }

    pub fn with_context<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut glow::Context) -> R,
    {
        info!("with_context called");
        let mut guard = self
            .inner
            .try_lock()
            .expect("Context should never be contended!");

        if guard.is_none() {
            info!("Creating context");
            fn load_gl_func(symbol_name: &str) -> *const c_void {
                info!("Looking up {symbol_name}");
                let c_str = CString::new(symbol_name).unwrap();
                // SAFETY: function provided by android
                unsafe { eglGetProcAddress(c_str.as_ptr().cast()) }
            }
            let glow_context = unsafe { glow::Context::from_loader_function(load_gl_func) };

            let current_thread = std::thread::current().id();
            *guard = Some(GLContext {
                glow_context,
                current_thread,
            });
        }
        let context = guard.as_mut().unwrap();
        assert_eq!(std::thread::current().id(), context.current_thread);

        let r = f(&mut context.glow_context);

        info!("with_context finished");
        r
    }
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
