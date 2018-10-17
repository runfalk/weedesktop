use dbus::{BusType, Connection, Message};
use weechat::Result;

pub fn screensaver_is_active() -> Result<bool> {
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
