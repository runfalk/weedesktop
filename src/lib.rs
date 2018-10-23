#[macro_use]
extern crate cfg_if;
#[cfg(all(unix, not(target_os = "macos")))]
extern crate dbus;
extern crate libc;
#[macro_use]
extern crate weedesktop_macro;

mod ffi;
mod platform;
mod weechat;

use platform::screensaver_is_active;
use std::time::Duration;
use weechat::{Buffer, CallResult, Plugin};

#[plugin_info]
pub static NAME: &str = "weedesktop";
#[plugin_info]
pub static DESCRIPTION: &str = "Desktop integration for Gnome. Supports auto-away.";
#[plugin_info]
pub static AUTHOR: &str = "Andreas Runfalk";
#[plugin_info]
pub static VERSION: &str = "0.1";
#[plugin_info]
pub static LICENSE: &str = "MIT";

fn check_screensaver(plugin: &Plugin, _remaining_calls: i32) -> CallResult {
    let screensaver_on = match screensaver_is_active() {
        Ok(is_on) => is_on,
        Err(_) => return Ok(()),
    };

    for irc_server in plugin
        .hdata_from_list("irc_server", "irc_servers")?
        .try_iter()?
    {
        let is_away = match irc_server.get_i32("is_away") {
            Ok(away) => away != 0,
            Err(_) => continue,
        };
        if let Ok(buffer_hdata) = irc_server.get_hdata("buffer") {
            let buffer = Buffer::try_from_hdata(buffer_hdata)?;
            if !is_away && screensaver_on {
                buffer.command("/away away").ok();
            } else if is_away && !screensaver_on {
                // Remove away status
                buffer.command("/away").ok();
            }
        }
    }
    Ok(())
}

fn open_url(_plugin: &Plugin, buffer: Buffer, _cmd: &str, _args: Vec<&str>) -> CallResult {
    buffer.print(buffer.get_name()?);
    Ok(())
}

#[plugin_init]
fn init(plugin: &Plugin) -> CallResult {
    plugin.hook_timer(Duration::from_secs(60), 0, check_screensaver)?;
    plugin.hook_command(
        "openurl",
        "Opens the most recent URL in the current buffer",
        None,
        None,
        None,
        open_url,
    )?;
    Ok(())
}
