//! ::  Project Path  ->  ep_start :: runtime.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 02:20 周六


use crate::config::ConfigStore;
use crate::renderer::Renderer;
use crate::window::{ StartController, WindowHost };
use configuration::StartPreferences;
use platform::PlatformRuntime;


pub struct StartRuntime {
	_window: WindowHost,
}


impl StartRuntime {
	pub fn new( platform: &PlatformRuntime, preferences: StartPreferences ) -> Result< Self, String > {
		let config_store = ConfigStore::discover()?;
		let config = config_store.load()?;
		let renderer = Renderer::new().map_err( |error| format!( "初始化 Direct2D/DirectWrite 失败：{}", error ) )?;
		let window = WindowHost::create( config_store, config, preferences, renderer, platform.input() )?;
		Ok( Self { _window: window } )
	}


	pub fn controller( &self ) -> StartController {
		self._window.controller()
	}
}
