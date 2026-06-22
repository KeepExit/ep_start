//! ::  Project Path  ->  ep_start :: lib.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 03:17 周六


mod dialog;
mod focus;
mod foreground;
mod input;
mod message_loop;
mod monitor;
mod process;
mod runtime;
mod tray;


pub use dialog::show_error_dialog;
pub use focus::ForegroundActivation;
pub use foreground::ForegroundChangeObserver;
pub use input::{ GlobalAltTabEvent, GlobalInputAction, GlobalInputBinding, GlobalInputManager, GlobalStartShortcut };
pub use monitor::MonitorGeometry;
pub use process::{ ensure_elevated, launch_shell_restart_helper, run_shell_restart_helper_if_requested, trim_working_set };
pub use runtime::PlatformRuntime;
pub use tray::{ EmbeddedIcon, TrayEvent, TrayIcon, TrayIconConfig, TrayMenuEntry };
