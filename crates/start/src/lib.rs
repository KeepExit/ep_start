//! ::  Project Path  ->  ep_start :: lib.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 02:39 周六


mod animation;
mod backdrop;
mod backdrop_capture;
mod config;
mod context_menu;
mod launcher;
mod layout;
mod overlay;
mod renderer;
mod runtime;
mod tile_customization;
mod transition;
mod window;


pub use runtime::StartRuntime;
pub use window::StartController;
