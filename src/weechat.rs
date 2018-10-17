use std::ffi::{CStr, CString, c_void};
use std::ptr;
use std::time::Duration;
use ffi;

pub type Result<T> = std::result::Result<T, ()>;

pub type CallResult = Result<()>;

#[derive(Debug)]
pub struct Plugin {
    pub ptr: *mut ffi::t_weechat_plugin,
}

type Hook = *mut ffi::t_hook;

type TimerHook = fn(&Plugin, i32) -> CallResult;

#[derive(Clone, Debug)]
pub struct Buffer<'a> {
    plugin: &'a Plugin,
    ptr: *mut ffi::t_gui_buffer,
}

#[derive(Clone, Debug)]
pub struct Hdata<'a> {
    plugin: &'a Plugin,
    ptr: *mut ffi::t_hdata,
}

pub struct BoundHdata<'a> {
    hdata: Hdata<'a>,
    ptr: *mut c_void,
}

#[derive(Debug)]
pub enum HdataValue<'a> {
    Char(u8),
    Int(i32),
    Long(i64),
    Str(&'a CStr),
    Ptr(*mut c_void),
    Time(libc::time_t),
    Hashtable(*mut ffi::t_hashtable),  // TODO: Implement Hashtable as type
    None,
}

macro_rules! call_attr {
    ($ptr:expr, $attr:ident $(,$arg:expr)*) => {
        match unsafe { (*$ptr).$attr } {
            Some(f) => unsafe { f($($arg),*) },
            None => unreachable!(),
        }
    };
}

macro_rules! try_ptr {
    ($ptr:expr) => {
        {
            // We must evaluate $ptr here or we will run the expression twice
            let ptr = $ptr;
            if ptr.is_null() {
                return Err(());
            } else {
                ptr
            }
        }
    };
}

impl<'a> Buffer<'a> {
    fn new(plugin: &'a Plugin, ptr: *mut ffi::t_gui_buffer) -> Self {
        Self { plugin: plugin, ptr }
    }

    pub fn command(&self, cmd: &str) -> CallResult {
        let ccmd = CString::new(cmd).unwrap();
        match call_attr!(self.plugin.ptr, command, self.plugin.ptr, self.ptr, ccmd.as_ptr()) {
            ffi::WEECHAT_RC_OK => Ok(()),
            ffi::WEECHAT_RC_ERROR => Err(()),
            _ => unreachable!(),
        }
    }

    pub fn hdata(&self) -> Result<BoundHdata> {
        Ok(self.plugin.hdata_get("buffer")?.bind(self.ptr as *mut c_void))
    }
}

impl<'a> Hdata<'a> {
    fn new(plugin: &'a Plugin, ptr: *mut ffi::t_hdata) -> Self {
        Self { plugin: plugin, ptr }
    }

    pub fn bind_list(&self, list_name: &str) -> Result<BoundHdata> {
        let clname = CString::new(list_name).or(Err(()))?;
        let ptr = try_ptr!(call_attr!(self.plugin.ptr, hdata_get_list, self.ptr, clname.as_ptr()));
        Ok(self.bind(ptr as *mut c_void))
    }

    pub fn bind(&self, ptr: *mut c_void) -> BoundHdata<'a> {
        BoundHdata { hdata: self.clone(), ptr }
    }

    fn get_type(&self, name: &str) -> i32 {
        let cname = match CString::new(name) {
            Ok(cstr) => cstr,
            Err(_) => return -1,
        };
        call_attr!(
            self.plugin.ptr,
            hdata_get_var_type,
            self.ptr,
            cname.as_ptr()
        )
    }
}

impl<'a> BoundHdata<'a> {
    pub fn get(&self, name: &str) -> HdataValue {
        let cname = match CString::new(name) {
            Ok(cstr) => cstr,
            Err(_) => return HdataValue::None,
        };
        // TODO: Support all types
        match self.hdata.get_type(name) {
            ffi::WEECHAT_HDATA_CHAR => {
                let r = call_attr!(
                    self.hdata.plugin.ptr,
                    hdata_char,
                    self.hdata.ptr,
                    self.ptr,
                    cname.as_ptr()
                );
                HdataValue::Char(r as u8)
            },
            ffi::WEECHAT_HDATA_INTEGER => {
                let r = call_attr!(
                    self.hdata.plugin.ptr,
                    hdata_integer,
                    self.hdata.ptr,
                    self.ptr,
                    cname.as_ptr()
                );
                HdataValue::Int(r)
            },
            ffi::WEECHAT_HDATA_LONG => {
                let r = call_attr!(
                    self.hdata.plugin.ptr,
                    hdata_long,
                    self.hdata.ptr,
                    self.ptr,
                    cname.as_ptr()
                );
                HdataValue::Long(r)
            },
            ffi::WEECHAT_HDATA_STRING | ffi::WEECHAT_HDATA_SHARED_STRING => {
                let r = call_attr!(
                    self.hdata.plugin.ptr,
                    hdata_string,
                    self.hdata.ptr,
                    self.ptr,
                    cname.as_ptr()
                );
                HdataValue::Str(unsafe { CStr::from_ptr(r) })
            },
            ffi::WEECHAT_HDATA_POINTER | ffi::WEECHAT_HDATA_OTHER => {
                let r = call_attr!(
                    self.hdata.plugin.ptr,
                    hdata_pointer,
                    self.hdata.ptr,
                    self.ptr,
                    cname.as_ptr()
                );
                HdataValue::Ptr(r)
            },
            ffi::WEECHAT_HDATA_TIME => {
                let r = call_attr!(
                    self.hdata.plugin.ptr,
                    hdata_time,
                    self.hdata.ptr,
                    self.ptr,
                    cname.as_ptr()
                );
                HdataValue::Time(r)
            },
            ffi::WEECHAT_HDATA_HASHTABLE => {
                let r = call_attr!(
                    self.hdata.plugin.ptr,
                    hdata_hashtable,
                    self.hdata.ptr,
                    self.ptr,
                    cname.as_ptr()
                );
                HdataValue::Hashtable(r)
            },
            _ => HdataValue::None,
        }
    }
}

pub extern "C" fn hook_timer_callback(ptr: *const c_void, data: *mut c_void, remaining_calls: i32) -> i32 {
    if data.is_null() {
        return ffi::WEECHAT_RC_ERROR;
    }

    let callback = unsafe { *(data as *mut TimerHook) };
    match callback(&Plugin::new(ptr as *mut ffi::t_weechat_plugin), remaining_calls) {
        Ok(_) => ffi::WEECHAT_RC_OK,
        Err(_) => ffi::WEECHAT_RC_ERROR,
    }
}

impl Plugin {
    pub fn new(ptr: *mut ffi::t_weechat_plugin) -> Self {
        Self { ptr }
    }

    pub fn print(&self, msg: &str) {
        let cmsg = CString::new(msg).unwrap();
        call_attr!(
            self.ptr,
            printf_date_tags,
            ptr::null_mut(),
            0,
            ptr::null(),
            cmsg.as_ptr()
        );
    }

    pub fn debug_print(&self, level: i32, msg: &str) {
        if unsafe { !(*self.ptr).debug >= level } {
            return;
        }
        self.print(msg);
    }

    pub fn hook_timer(&self, interval: Duration, max_calls: i32, callback: TimerHook) -> Result<Hook> {
        // Allocate a blob big enough to hold a pointer to a function. This will be
        // used to allow hook_timer_callback to dispatch the callback to the given
        // TimerHook. We must use malloc since Weechat will automatically free the
        // pointer we give when the plugin is tearing down
        let callback_ptr = try_ptr!(unsafe {
             libc::malloc(std::mem::size_of::<TimerHook>()) as *mut TimerHook
        });

        // Assign function pointer to the datablob that is sent to the callback hook
        unsafe {
            *callback_ptr = callback;
        }

        Ok(try_ptr!(call_attr!(
            self.ptr,
            hook_timer,
            self.ptr,
            (1000 * interval.as_secs() + interval.subsec_millis() as u64) as i64,
            0,
            max_calls,
            Some(hook_timer_callback),
            self.ptr as *const c_void,
            callback_ptr as *mut c_void
        )))
    }

    pub fn buffer_search_main(&self) -> Option<Buffer> {
        let ptr = call_attr!(self.ptr, buffer_search_main);
        if ptr.is_null() {
            None
        } else {
            Some(Buffer::new(&self, ptr))
        }
    }

    pub fn hdata_get(&self, name: &str) -> Result<Hdata> {
        let cname = CString::new(name).or(Err(()))?;
        let hdata_ptr = try_ptr!(call_attr!(self.ptr, hdata_get, self.ptr, cname.as_ptr()));
        Ok(Hdata::new(&self, hdata_ptr))
    }
}
