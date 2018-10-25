use super::{Plugin, Result};
use std::ffi::{c_void, CStr, CString};
use std::ptr;

#[derive(Clone, Debug)]
pub struct Hdata<'a> {
    pub(crate) plugin: &'a Plugin,
    pub(crate) hdata_ptr: *mut ::ffi::t_hdata,
    pub(crate) data_ptr: *mut c_void,
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
    Hashtable(*mut ::ffi::t_hashtable), // TODO: Implement Hashtable as type
    None,
}

impl<'a> Hdata<'a> {
    pub(crate) fn new(
        plugin: &'a Plugin,
        hdata_ptr: *mut ::ffi::t_hdata,
        data_ptr: *mut c_void,
    ) -> Self {
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
            ::ffi::WEECHAT_HDATA_OTHER => {
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
            ::ffi::WEECHAT_HDATA_CHAR => {
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
            ::ffi::WEECHAT_HDATA_INTEGER => {
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
            ::ffi::WEECHAT_HDATA_LONG => {
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
            ::ffi::WEECHAT_HDATA_STRING | ::ffi::WEECHAT_HDATA_SHARED_STRING => {
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
            ::ffi::WEECHAT_HDATA_POINTER => {
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
            ::ffi::WEECHAT_HDATA_TIME => {
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
            ::ffi::WEECHAT_HDATA_HASHTABLE => {
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

    pub fn get(&self, name: &str) -> HdataValue<'a> {
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

    pub fn get_cstr(&self, name: &str) -> Result<&'a CStr> {
        match self.get(name) {
            HdataValue::Str(v) => Ok(&v),
            _ => Err(()),
        }
    }

    pub fn get_str(&self, name: &str) -> Result<&'a str> {
        self.get_cstr(name)?.to_str().or(Err(()))
    }

    pub fn get_hdata(&self, name: &str) -> Result<Hdata<'a>> {
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
