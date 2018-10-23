#![allow(dead_code)]

use ffi;
use std::ffi::{c_void, CString};
use std::ptr;
use std::time::Duration;

#[macro_use]
mod macros;

mod callbacks;
mod hdata;

use self::callbacks::{malloc_callback, TimerHook};
pub use self::hdata::Hdata;

pub type Result<T> = std::result::Result<T, ()>;

pub type CallResult = Result<()>;

#[derive(Debug)]
pub struct Plugin {
    pub ptr: *mut ffi::t_weechat_plugin,
}

type Hook = *mut ffi::t_hook;

#[derive(Clone, Debug)]
pub struct Buffer<'a> {
    plugin: &'a Plugin,
    ptr: *mut ffi::t_gui_buffer,
}


impl<'a> Buffer<'a> {
    fn new(plugin: &'a Plugin, ptr: *mut ffi::t_gui_buffer) -> Self {
        Self { plugin, ptr }
    }

    pub fn from_hdata(hdata: &'a Hdata) -> Self {
        Self {
            plugin: hdata.plugin,
            ptr: hdata.data_ptr as *mut ffi::t_gui_buffer,
        }
    }

    pub fn command(&self, cmd: &str) -> CallResult {
        let ccmd = CString::new(cmd).unwrap();
        let result = unsafe {
            call_attr!(
                self.plugin.ptr,
                command,
                self.plugin.ptr,
                self.ptr,
                ccmd.as_ptr()
            )
        };
        match result {
            ffi::WEECHAT_RC_OK => Ok(()),
            ffi::WEECHAT_RC_ERROR => Err(()),
            _ => unreachable!(),
        }
    }

    pub fn hdata(&self) -> Result<Hdata> {
        Ok(self
            .plugin
            .hdata_from_ptr("buffer", self.ptr as *mut c_void)?)
    }
}

impl Plugin {
    pub fn new(ptr: *mut ffi::t_weechat_plugin) -> Self {
        Self { ptr }
    }

    pub fn print(&self, msg: &str) {
        let cmsg = CString::new(msg).unwrap();
        unsafe {
            call_attr!(
                self.ptr,
                printf_date_tags,
                ptr::null_mut(),
                0,
                ptr::null(),
                cmsg.as_ptr()
            );
        }
    }

    pub fn debug_print(&self, level: i32, msg: &str) {
        if unsafe { !(*self.ptr).debug >= level } {
            return;
        }
        self.print(msg);
    }

    pub fn hook_timer(
        &self,
        interval: Duration,
        max_calls: i32,
        callback: TimerHook,
    ) -> Result<Hook> {
        Ok(try_ptr!(unsafe {
            call_attr!(
                self.ptr,
                hook_timer,
                self.ptr,
                (1000 * interval.as_secs() + interval.subsec_millis() as u64) as i64,
                0,
                max_calls,
                Some(callbacks::hook_timer),
                self.ptr as *const c_void,
                malloc_callback(callback)? as *mut c_void
            )
        }))
    }

    pub fn buffer_search_main(&self) -> Option<Buffer> {
        let ptr = unsafe { call_attr!(self.ptr, buffer_search_main) };
        if ptr.is_null() {
            None
        } else {
            Some(Buffer::new(&self, ptr))
        }
    }

    fn hdata_ptr(&self, name: &str) -> Result<*mut ffi::t_hdata> {
        let cname = CString::new(name).or(Err(()))?;
        let ptr = unsafe { call_attr!(self.ptr, hdata_get, self.ptr, cname.as_ptr()) };
        Ok(try_ptr!(ptr))
    }

    pub fn hdata_from_ptr(&self, name: &str, data_ptr: *mut c_void) -> Result<Hdata> {
        Ok(Hdata::new(&self, self.hdata_ptr(name)?, data_ptr))
    }

    pub fn hdata_from_list(&self, name: &str, list: &str) -> Result<Hdata> {
        let hdata_ptr = self.hdata_ptr(name)?;
        let clist = CString::new(list).or(Err(()))?;
        let data_ptr =
            try_ptr!(unsafe { call_attr!(self.ptr, hdata_get_list, hdata_ptr, clist.as_ptr()) });
        Ok(Hdata::new(&self, hdata_ptr, data_ptr))
    }
}
