//! Main-thread-only ownership boundary for AppKit preview views.
#![allow(unsafe_code)]

use gpui_kit::{Bounds, Pixels, Window};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{ffi::c_void, marker::PhantomData, ptr::NonNull, rc::Rc};

unsafe extern "C" {
    fn glim_preview_create(bytes: *const u8, length: usize) -> *mut c_void;
    fn glim_preview_layout(
        handle: *mut c_void,
        parent: *mut c_void,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    );
    fn glim_preview_hide(handle: *mut c_void);
    fn glim_preview_release(handle: *mut c_void);
}

pub(super) struct NativePreview {
    handle: NonNull<c_void>,
    // GPUI entities are created/updated/dropped on the main thread. Prevent this
    // owner from ever being moved into a background task.
    _main_thread: PhantomData<Rc<()>>,
}
impl NativePreview {
    pub fn new(configuration: &[u8]) -> Self {
        // SAFETY: Swift copies this valid slice before returning a retained handle.
        let handle = unsafe { glim_preview_create(configuration.as_ptr(), configuration.len()) };
        Self {
            handle: NonNull::new(handle).expect("Swift returned a null preview"),
            _main_thread: PhantomData,
        }
    }
    pub fn layout(&self, bounds: Bounds<Pixels>, window: &Window) {
        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return;
        };
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return;
        };
        // SAFETY: the live GPUI window owns the parent NSView. Swift retains the
        // child in that view; both it and our preview handle remain alive here.
        unsafe {
            glim_preview_layout(
                self.handle.as_ptr(),
                handle.ns_view.as_ptr(),
                f32::from(bounds.origin.x) as f64,
                f32::from(bounds.origin.y) as f64,
                f32::from(bounds.size.width) as f64,
                f32::from(bounds.size.height) as f64,
            )
        }
    }
    pub fn hide(&self) {
        // SAFETY: this owner retains the Swift controller, on the main thread.
        unsafe { glim_preview_hide(self.handle.as_ptr()) }
    }
}
impl Drop for NativePreview {
    fn drop(&mut self) {
        // SAFETY: release the sole retained handle exactly once, detaching its view
        // and stopping playback before the prepared SQLite snapshot is dropped.
        unsafe { glim_preview_release(self.handle.as_ptr()) }
    }
}
