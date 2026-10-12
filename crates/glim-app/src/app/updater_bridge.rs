//! Main-thread-only Sparkle ownership and C-string boundary.
#![allow(unsafe_code)]
use glim_core::update::UpdateSnapshot;
use std::{
    ffi::{CStr, CString, c_char, c_void},
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};

unsafe extern "C" {
    fn glim_updater_create() -> *mut c_void;
    fn glim_updater_snapshot(handle: *mut c_void) -> *mut c_char;
    fn glim_updater_command(handle: *mut c_void, command: *const c_char);
    fn glim_updater_free_string(string: *mut c_char);
    fn glim_updater_release(handle: *mut c_void);
}
pub(super) struct NativeUpdater {
    handle: NonNull<c_void>,
    _main_thread: PhantomData<Rc<()>>,
}
impl NativeUpdater {
    pub fn new() -> Self {
        // SAFETY: called only while initializing the GPUI app on the UI thread.
        let handle = unsafe { glim_updater_create() };
        Self {
            handle: NonNull::new(handle).expect("Null updater handle"),
            _main_thread: PhantomData,
        }
    }
    pub fn snapshot(&self) -> anyhow::Result<UpdateSnapshot> {
        // SAFETY: Swift returns a strdup-allocated, NUL-terminated string owned here.
        let ptr = unsafe { glim_updater_snapshot(self.handle.as_ptr()) };
        anyhow::ensure!(!ptr.is_null(), "Cannot read updater state");
        let result = unsafe { serde_json::from_slice(CStr::from_ptr(ptr).to_bytes()) };
        // SAFETY: exactly one matching free, after the JSON deserializer has copied data.
        unsafe { glim_updater_free_string(ptr) };
        Ok(result?)
    }
    pub fn command(&self, command: &str) {
        let command = CString::new(command).expect("Updater commands contain no NUL");
        // SAFETY: Swift consumes this borrowed string synchronously on the UI thread.
        unsafe { glim_updater_command(self.handle.as_ptr(), command.as_ptr()) }
    }
}
impl Drop for NativeUpdater {
    fn drop(&mut self) {
        // SAFETY: release the sole retained Swift owner exactly once on the UI thread.
        unsafe { glim_updater_release(self.handle.as_ptr()) }
    }
}
