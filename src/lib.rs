extern crate libc;
extern crate dbus;
#[macro_use]
extern crate weedesktop_macro;

mod ffi;
mod weechat;

use dbus::{BusType, Connection, Message};
use std::time::Duration;
use weechat::{CallResult, HdataValue, Plugin, Result};

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

fn screensaver_is_active() -> Result<bool> {
    let conn = Connection::get_private(BusType::Session).or(Err(()))?;
    let msg = Message::new_method_call(
        "org.gnome.ScreenSaver",
        "/org/gnome/ScreenSaver",
        "org.gnome.ScreenSaver",
        "GetActive",
    ).or(Err(()))?;

    let resp = conn.send_with_reply_and_block(msg, 100).or(Err(()))?;
    Ok(resp.get1().ok_or(())?)
}

fn is_away(plugin: &Plugin) -> Result<bool> {
    // TODO: We actually only check one server
    let hdata = plugin.hdata_get("irc_server").ok().ok_or(())?;
    let irc_server = hdata.bind_list("irc_servers")?;
    match irc_server.get("is_away") {
        HdataValue::Int(r) => Ok(r != 0),
        _ => Err(()),
    }
}

fn check_screensaver(plugin: &Plugin, _remaining_calls: i32) -> CallResult {
    let is_away = is_away(plugin)?;
    if let Ok(screensaver_on) = screensaver_is_active() {
        let buffer = plugin.buffer_search_main().ok_or(())?;
        if !is_away && screensaver_on {
            buffer.command("/allserv away Away")?;
        } else if is_away && !screensaver_on {
            // Remove away status
            buffer.command("/allserv away")?;
        }
    }
    Ok(())
}

#[plugin_init]
fn init(plugin: &Plugin) -> CallResult {
    plugin.hook_timer(Duration::from_secs(60), 0, check_screensaver)?;
    Ok(())
}
