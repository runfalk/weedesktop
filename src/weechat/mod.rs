#![allow(dead_code)]

use ffi;
use std::ffi::{c_void, CString};
use std::ptr;
use std::time::Duration;

#[macro_use]
mod macros;

mod callbacks;
mod hdata;

use self::callbacks::{malloc_callback, CommandHook, TimerHook};
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
    hdata: Hdata<'a>,
    ptr: *mut ffi::t_gui_buffer,
}

#[derive(Clone, Debug)]
pub struct LineIterator<'a> {
    next_key: String,
    next_hdata: Option<Hdata<'a>>,
}

impl<'a> Buffer<'a> {
    pub fn try_from_hdata(hdata: Hdata<'a>) -> Result<Self> {
        // Check if the pointer is a gui buffer pointer
        let buffer_list = hdata.plugin.hdata_from_list("buffer", "gui_buffers")?;
        if unsafe {
            call_attr!(
                hdata.plugin.ptr,
                hdata_check_pointer,
                buffer_list.hdata_ptr,
                buffer_list.data_ptr,
                hdata.data_ptr
            )
        } == 0
        {
            return Err(());
        }
        Ok(Self {
            ptr: hdata.data_ptr as *mut ffi::t_gui_buffer,
            hdata,
        })
    }

    pub fn get_name(&self) -> Result<&'a str> {
        self.hdata.get_str("name")
    }

    pub fn command(&self, cmd: &str) -> CallResult {
        let ccmd = CString::new(cmd).unwrap();
        let result = unsafe {
            call_attr!(
                self.hdata.plugin.ptr,
                command,
                self.hdata.plugin.ptr,
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

    pub fn print(&self, msg: &str) {
        let cmsg = CString::new(msg).unwrap();
        unsafe {
            call_attr!(
                self.hdata.plugin.ptr,
                printf_date_tags,
                self.ptr,
                0,
                ptr::null(),
                cmsg.as_ptr()
            );
        }
    }

    pub fn iter_lines_from_top(&self) -> Result<LineIterator<'a>> {
        Ok(LineIterator {
            next_key: "next_line".to_owned(),
            next_hdata: self.hdata.get_hdata("lines")?.get_hdata("first_line").ok(),
        })
    }

    pub fn iter_lines_from_bottom(&self) -> Result<LineIterator<'a>> {
        Ok(LineIterator {
            next_key: "prev_line".to_owned(),
            next_hdata: self.hdata.get_hdata("lines")?.get_hdata("last_line").ok(),
        })
    }
}

impl<'a> Iterator for LineIterator<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next_hdata.take();
        self.next_hdata = match current {
            Some(ref c) => c.get_hdata(&self.next_key).ok(),
            None => None,
        };

        if let Some(c) = current {
            Some(c.get_hdata("data").unwrap().get_str("message").unwrap())
        } else {
            None
        }
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

    pub fn hook_command<'a, D, A, H, C>(
        &self,
        cmd: &str,
        description: D,
        args: A,
        args_help: H,
        completion: C,
        callback: CommandHook,
    ) -> Result<Hook>
    where
        D: Into<Option<&'a str>>,
        A: Into<Option<&'a str>>,
        H: Into<Option<&'a str>>,
        C: Into<Option<&'a str>>,
    {
        let ccmd = CString::new(cmd).or(Err(()))?;
        let cdescription = CString::new(description.into().unwrap_or("")).or(Err(()))?;
        let cargs = CString::new(args.into().unwrap_or("")).or(Err(()))?;
        let cargs_help = CString::new(args_help.into().unwrap_or("")).or(Err(()))?;
        let ccompletion = CString::new(completion.into().unwrap_or("")).or(Err(()))?;

        Ok(try_ptr!(unsafe {
            call_attr!(
                self.ptr,
                hook_command,
                self.ptr,
                ccmd.as_ptr(),
                cdescription.as_ptr(),
                cargs.as_ptr(),
                cargs_help.as_ptr(),
                ccompletion.as_ptr(),
                Some(callbacks::hook_command),
                self.ptr as *const c_void,
                malloc_callback(callback)? as *mut c_void
            )
        }))
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
            return None;
        }

        let hdata = match self.hdata_from_ptr("buffer", ptr as *mut c_void).ok() {
            Some(h) => h,
            None => return None,
        };

        Buffer::try_from_hdata(hdata).ok()
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
