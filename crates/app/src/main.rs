//! ::  Project Path  ->  ep_start :: main.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/19 22:30 周五


#![windows_subsystem = "windows"]

fn main() {
	if let Err( error ) = app::run() { app::show_fatal_error( &error ); }
}
