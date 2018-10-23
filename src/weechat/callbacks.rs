use std::ffi::c_void;
use super::{CallResult, Plugin};

pub type TimerHook = fn(&Plugin, i32) -> CallResult;

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
