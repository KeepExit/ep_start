//! ::  Project Path  ->  ep_start :: window.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 20:46 周六


use crate::state::SettingsState;
use crate::host::{SettingsWindowHost, post_show_settings };
use configuration::{ AppPreferences, ConfigurationStore, StartPreferences };
use localization::TextResources;
use platform::EmbeddedIcon;
use windows::Win32::Foundation::HWND;


pub struct SettingsRuntime {
	window: SettingsWindow,
}

#[derive( Clone, Copy )]
pub struct SettingsController {
	hwnd: HWND,
}

struct SettingsWindow {
	host: SettingsWindowHost,
	state: *mut SettingsState,
	_large_icon: EmbeddedIcon,
	_small_icon: EmbeddedIcon,
}

impl SettingsRuntime {
	pub fn new( store: ConfigurationStore, preferences: AppPreferences, small_icon: &'static [ u8 ], large_icon: &'static [ u8 ], on_change: impl FnMut( StartPreferences ) + 'static ) -> Result< Self, String > {
		Ok( Self { window: SettingsWindow::create( store, preferences, small_icon, large_icon, on_change )? } )
	}
	pub fn controller( &self ) -> SettingsController {
		SettingsController { hwnd: self.window.host.hwnd() }
	}
}

impl SettingsController {
	pub fn show( &self ) {
		post_show_settings( self.hwnd );
	}
}

impl SettingsWindow {
	fn create( store: ConfigurationStore, preferences: AppPreferences, small_icon_source: &'static [ u8 ], large_icon_source: &'static [ u8 ], on_change: impl FnMut( StartPreferences ) + 'static ) -> Result< Self, String > {
		let large_icon = EmbeddedIcon::load_for_size( large_icon_source, 32, 32 )?;
		let small_icon = EmbeddedIcon::load_for_size( small_icon_source, 16, 16 )?;
		let text = TextResources::system()?;
		let state = Box::into_raw( Box::new( SettingsState::new( store, preferences, text, on_change ) ) );
		let host = match unsafe { SettingsWindowHost::create( state, large_icon.handle(), small_icon.handle() ) } {
			Ok( host ) => host,
			Err( error ) => {
				unsafe { drop( Box::from_raw( state ) ); }
				return Err( format!( "创建设置窗口失败：{}", error ) );
			}
		};
		Ok( Self { host, state, _large_icon: large_icon, _small_icon: small_icon } )
	}
}

impl Drop for SettingsWindow {
	fn drop( &mut self ) {
		unsafe {
			self.host.destroy();
			drop( Box::from_raw( self.state ) );
		}
	}
}