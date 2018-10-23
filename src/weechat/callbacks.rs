use std::ffi::c_void;
use super::{CallResult, Plugin, Result};

pub type TimerHook = fn(&Plugin, i32) -> CallResult;

pub fn malloc_callback<T>(callback: T) -> Result<*mut T> {
    // Allocate a blob big enough to hold a pointer to a function. This will be
    // used to allow hook_timer_callback to dispatch the callback to the given
    // TimerHook. We must use malloc since Weechat will automatically free the
    // pointer we give when the plugin is tearing down
    let callback_ptr = try_ptr!(unsafe { libc::malloc(std::mem::size_of::<T>()) as *mut T });

    // Assign function pointer to the datablob that is sent to the callback hook
    unsafe {
        *callback_ptr = callback;
    }

    Ok(callback_ptr)
}

pub extern "C" fn hook_timer(
    ptr: *const c_void,
    data: *mut c_void,
    remaining_calls: i32,
) -> i32 {
    if data.is_null() {
        return ::ffi::WEECHAT_RC_ERROR;
    }

    let callback = unsafe { *(data as *mut TimerHook) };
    match callback(
        &Plugin::new(ptr as *mut ::ffi::t_weechat_plugin),
        remaining_calls,
    ) {
        Ok(_) => ::ffi::WEECHAT_RC_OK,
        Err(_) => ::ffi::WEECHAT_RC_ERROR,
    }
}
