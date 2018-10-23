#[macro_export]
macro_rules! call_attr {
    ($ptr:expr, $attr:ident $(,$arg:expr)*) => {
        match (*$ptr).$attr {
            Some(f) => f($($arg),*),
            None => unreachable!(),
        }
    };
}

#[macro_export]
macro_rules! try_ptr {
    ($ptr:expr) => {{
        // We must evaluate $ptr here or we will run the expression twice
        let ptr = $ptr;
        if ptr.is_null() {
            return Err(());
        } else {
            ptr
        }
    }};
}
