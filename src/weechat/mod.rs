#![allow(dead_code)]

use ffi;
use std::ffi::{c_void, CStr, CString};
use std::ptr;
use std::time::Duration;

mod callbacks;
#[macro_use]
mod macros;

use self::callbacks::TimerHook;

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

#[derive(Clone, Debug)]
pub struct Hdata<'a> {
    plugin: &'a Plugin,
    hdata_ptr: *mut ffi::t_hdata,
    data_ptr: *mut c_void,
}

#[derive(Clone, Debug)]
pub struct HdataIterator<'a> {
    next_hdata: Option<Hdata<'a>>,
    next_key: &'a CStr,
}

#[derive(Clone, Debug)]
pub enum HdataValue<'a> {
    Other(*mut c_void),
    I8(i8),
    I32(i32),
    I64(i64),
    Str(&'a CStr),
    Ptr(*mut c_void),
    Hdata(Hdata<'a>),
    Time(libc::time_t),
    Hashtable(*mut ffi::t_hashtable), // TODO: Implement Hashtable as type
    None,
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

impl<'a> Hdata<'a> {
    fn new(plugin: &'a Plugin, hdata_ptr: *mut ffi::t_hdata, data_ptr: *mut c_void) -> Self {
        Self {
            plugin,
            hdata_ptr,
            data_ptr,
        }
    }

    fn get_type_from_cstr(&self, cname: &CStr) -> i32 {
        unsafe {
            call_attr!(
                self.plugin.ptr,
                hdata_get_var_type,
                self.hdata_ptr,
                cname.as_ptr()
            )
        }
    }

    fn get_from_cstr(&self, cname: &CStr) -> HdataValue<'a> {
        match self.get_type_from_cstr(cname) {
            ffi::WEECHAT_HDATA_OTHER => {
                let ptr = unsafe {
                    call_attr!(
                        self.plugin.ptr,
                        hdata_get_var,
                        self.hdata_ptr,
                        self.data_ptr,
                        cname.as_ptr()
                    )
                };
                HdataValue::Other(ptr)
            },
            ffi::WEECHAT_HDATA_CHAR => {
                let chr = unsafe {
                    call_attr!(
                        self.plugin.ptr,
                        hdata_char,
                        self.hdata_ptr,
                        self.data_ptr,
                        cname.as_ptr()
                    )
                };
                HdataValue::I8(chr)
            },
            ffi::WEECHAT_HDATA_INTEGER => {
                let int = unsafe {
                    call_attr!(
                        self.plugin.ptr,
                        hdata_integer,
                        self.hdata_ptr,
                        self.data_ptr,
                        cname.as_ptr()
                    )
                };
                HdataValue::I32(int)
            },
            ffi::WEECHAT_HDATA_LONG => {
                let long = unsafe {
                    call_attr!(
                        self.plugin.ptr,
                        hdata_long,
                        self.hdata_ptr,
                        self.data_ptr,
                        cname.as_ptr()
                    )
                };
                HdataValue::I64(long)
            },
            ffi::WEECHAT_HDATA_STRING | ffi::WEECHAT_HDATA_SHARED_STRING => {
                let char_ptr = unsafe {
                    call_attr!(
                        self.plugin.ptr,
                        hdata_string,
                        self.hdata_ptr,
                        self.data_ptr,
                        cname.as_ptr()
                    )
                };
                HdataValue::Str(unsafe { CStr::from_ptr(char_ptr) })
            },
            ffi::WEECHAT_HDATA_POINTER => {
                let ptr = unsafe {
                    call_attr!(
                        self.plugin.ptr,
                        hdata_pointer,
                        self.hdata_ptr,
                        self.data_ptr,
                        cname.as_ptr()
                    )
                };
                if ptr == ptr::null_mut() {
                    return HdataValue::None;
                }
                let hdata_name_ptr = unsafe {
                    call_attr!(
                        self.plugin.ptr,
                        hdata_get_var_hdata,
                        self.hdata_ptr,
                        cname.as_ptr()
                    )
                };
                if hdata_name_ptr == ptr::null_mut() {
                    return HdataValue::Ptr(ptr);
                }
                let hdata_cname = unsafe { CStr::from_ptr(hdata_name_ptr) };
                match self
                    .plugin
                    .hdata_from_ptr(hdata_cname.to_str().unwrap(), ptr)
                {
                    Ok(h) => HdataValue::Hdata(h),
                    Err(_) => HdataValue::None,
                }
            },
            ffi::WEECHAT_HDATA_TIME => {
                let time = unsafe {
                    call_attr!(
                        self.plugin.ptr,
                        hdata_time,
                        self.hdata_ptr,
                        self.data_ptr,
                        cname.as_ptr()
                    )
                };
                HdataValue::Time(time)
            },
            ffi::WEECHAT_HDATA_HASHTABLE => {
                let hashtable_ptr = unsafe {
                    call_attr!(
                        self.plugin.ptr,
                        hdata_hashtable,
                        self.hdata_ptr,
                        self.data_ptr,
                        cname.as_ptr()
                    )
                };
                HdataValue::Hashtable(hashtable_ptr)
            },
            _ => HdataValue::None,
        }
    }

    pub fn get(&self, name: &str) -> HdataValue {
        let cname = match CString::new(name) {
            Ok(cstr) => cstr,
            Err(_) => return HdataValue::None,
        };
        self.get_from_cstr(cname.as_c_str())
    }

    pub fn get_i8(&self, name: &str) -> Result<i8> {
        match self.get(name) {
            HdataValue::I8(v) => Ok(v),
            _ => Err(()),
        }
    }

    pub fn get_i32(&self, name: &str) -> Result<i32> {
        match self.get(name) {
            HdataValue::I32(v) => Ok(v),
            _ => Err(()),
        }
    }

    pub fn get_i64(&self, name: &str) -> Result<i64> {
        match self.get(name) {
            HdataValue::I64(v) => Ok(v),
            _ => Err(()),
        }
    }

    pub fn get_cstr(&'a self, name: &str) -> Result<&'a CStr> {
        match self.get(name) {
            HdataValue::Str(v) => Ok(&v),
            _ => Err(()),
        }
    }

    pub fn get_str(&'a self, name: &str) -> Result<&'a str> {
        self.get_cstr(name)?.to_str().or(Err(()))
    }

    pub fn get_hdata(&'a self, name: &str) -> Result<Hdata<'a>> {
        match self.get(name) {
            HdataValue::Hdata(v) => Ok(v),
            _ => Err(()),
        }
    }

    pub fn try_iter(&self) -> Result<HdataIterator<'a>> {
        let cnext = CString::new("var_next").or(Err(()))?;
        let var_next = unsafe {
            CStr::from_ptr(try_ptr!(call_attr!(
                self.plugin.ptr,
                hdata_get_string,
                self.hdata_ptr,
                cnext.as_ptr()
            )))
        };

        Ok(HdataIterator {
            next_hdata: Some(self.clone()),
            next_key: &var_next,
        })
    }
}

impl<'a> Iterator for HdataIterator<'a> {
    type Item = Hdata<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next_hdata.take();
        if let Some(ref c) = current {
            self.next_hdata = match c.get_from_cstr(self.next_key) {
                HdataValue::Hdata(hdata) => Some(hdata),
                _ => None,
            };
        };
        current
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
        // Allocate a blob big enough to hold a pointer to a function. This will be
        // used to allow hook_timer_callback to dispatch the callback to the given
        // TimerHook. We must use malloc since Weechat will automatically free the
        // pointer we give when the plugin is tearing down
        let callback_ptr =
            try_ptr!(unsafe { libc::malloc(std::mem::size_of::<TimerHook>()) as *mut TimerHook });

        // Assign function pointer to the datablob that is sent to the callback hook
        unsafe {
            *callback_ptr = callback;
        }

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
                callback_ptr as *mut c_void
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
