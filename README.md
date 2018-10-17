Weedesktop
==========
Desktop integration for Weechat. It currently handles:

* Auto-away when screen is locked (Gnome shell only)

Planned features:

* macOS support
* Notification integration
* Opening URLs, `/openurl` will open the latest URL in your prefered browser

Installation
------------
You can probably install it using:

```bash
cargo install --git https://github.com/runfalk/weedesktop.git
```

Note that you need `weechat-plugin.h` and clang installed. On most package
managers you can install `weechat-dev` or `weechat-devel` for the development
headers.
