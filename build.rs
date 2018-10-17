extern crate bindgen;

use bindgen::callbacks::{IntKind, ParseCallbacks};
use std::env;
use std::path::PathBuf;

#[derive(Debug)]
struct Fixer;
impl ParseCallbacks for Fixer {
    fn int_macro(&self, name: &str, _value: i64) -> Option<IntKind> {
        match name {
            "WEECHAT_HDATA_OTHER"
            | "WEECHAT_HDATA_CHAR"
            | "WEECHAT_HDATA_INTEGER"
            | "WEECHAT_HDATA_LONG"
            | "WEECHAT_HDATA_STRING"
            | "WEECHAT_HDATA_POINTER"
            | "WEECHAT_HDATA_TIME"
            | "WEECHAT_HDATA_HASHTABLE"
            | "WEECHAT_HDATA_SHARED_STRING"
            | "WEECHAT_HDATA_LIST_CHECK_POINTERS"
            | "WEECHAT_RC_OK"
            | "WEECHAT_RC_OK_EAT" => Some(IntKind::I32),
            _ => None,
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn get_weechat_inc_dir() -> PathBuf {
    PathBuf::from("/usr/include/weechat")
}

#[cfg(target_os = "macos")]
fn get_weechat_inc_dir() -> PathBuf {
    // We assume Homebrew is installed since there aren't any more probable
    // alternatives
    let output = std::process::Command::new("brew")
        .arg("--prefix")
        .arg("weechat")
        .output()
        .expect("Failed to locate Weechat using Homebrew");
    let string = String::from_utf8(output.stdout).expect("Failed to parse brew output");
    PathBuf::from(string.trim_right()).join("include/weechat")
}

fn main() {
    let inc_path = match env::var("WEECHAT_INC_DIR") {
        Ok(p) => PathBuf::from(p),
        Err(_) => get_weechat_inc_dir(),
    };
    let bindings = bindgen::Builder::default()
        .header(inc_path.join("weechat-plugin.h").to_str().unwrap())
        .layout_tests(false)
        .parse_callbacks(Box::new(Fixer {}))
        .rustfmt_bindings(true)
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("weechat_plugin.rs"))
        .expect("Couldn't write bindings!");
}
