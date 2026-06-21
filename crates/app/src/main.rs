//! ::  Project Path  ->  ep_start :: main.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/19 22:30 周五


#![windows_subsystem = "windows"]

use app::injector_bridge;


fn main() {
	injector_bridge::ensure_injected();

	run_app();
}


fn run_app() {
	println!( "ep_start running..." );
}