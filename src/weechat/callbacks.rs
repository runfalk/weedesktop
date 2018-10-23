use super::{Buffer, CallResult, Plugin, Result};
use std::ffi::{c_void, CStr};

macro_rules! try_unwrap {
    ($expr:expr) => {
        match $expr {
            Ok(x) => x,
            Err(_) => return ::ffi::WEECHAT_RC_ERROR,
        }
    };
}

pub type CommandHook = fn(&Plugin, buffer: Buffer, cmd: &str, args: Vec<&str>) -> CallResult;
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

pub extern "C" fn hook_command(
    ptr: *const c_void,
    data: *mut c_void,
    buffer: *mut ::ffi::t_gui_buffer,
    argc: i32,
    argv: *mut *mut i8,
    _argv_eol: *mut *mut i8,
) -> i32 {
    if data.is_null() || argc < 1 {
        return ::ffi::WEECHAT_RC_ERROR;
    }

    let plugin = Plugin::new(ptr as *mut ::ffi::t_weechat_plugin);
    let hdata = try_unwrap!(plugin.hdata_from_ptr("buffer", buffer as *mut c_void));
    let buffer = try_unwrap!(Buffer::try_from_hdata(hdata));
    let cmd = try_unwrap!(unsafe { CStr::from_ptr(*argv).to_str() });

    // Since the first arg is the command name we start at 1 here
    let mut args: Vec<&str> = Vec::with_capacity((argc - 1) as usize);
    for i in 1..(argc as isize) {
        match unsafe { CStr::from_ptr(*argv.offset(i)).to_str() } {
            Ok(s) => args.push(s),
            Err(_) => return ::ffi::WEECHAT_RC_ERROR,
        };
    }

    let callback = unsafe { *(data as *mut CommandHook) };
    match callback(&plugin, buffer, cmd, args) {
        Ok(_) => ::ffi::WEECHAT_RC_OK,
        Err(_) => ::ffi::WEECHAT_RC_ERROR,
    }
}

pub extern "C" fn hook_timer(ptr: *const c_void, data: *mut c_void, remaining_calls: i32) -> i32 {
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
