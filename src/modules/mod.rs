pub mod audio;
pub mod crystal_manager;
pub mod curses;
pub mod presence;
pub mod songs;
pub mod utils;
pub mod general;
pub mod tui_ir;
pub mod mouse;


#[cfg(not(target_os = "windows"))]
pub mod dbus;
