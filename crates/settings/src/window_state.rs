//! ::  Project Path  ->  ep_start :: window_state.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 00:53 周日


use std::ffi::c_void;
use std::mem::size_of;
use windows::Win32::System::Registry::{ HKEY, HKEY_CURRENT_USER, REG_DWORD, RRF_RT_REG_DWORD, RegCloseKey, RegCreateKeyW, RegGetValueW, RegSetValueExW };
use windows::core::w;


const DEFAULT_WIDTH: i32 = 1100;
const DEFAULT_HEIGHT: i32 = 720;


#[derive( Clone, Copy )]
pub struct WindowSize {
	pub width: i32,
	pub height: i32,
}


pub struct WindowSizeStore;


impl WindowSizeStore {
	pub fn load() -> WindowSize {
		WindowSize { width: read_value( w!( "SettingsWidth" ) ).unwrap_or( DEFAULT_WIDTH as u32 ) as i32, height: read_value( w!( "SettingsHeight" ) ).unwrap_or( DEFAULT_HEIGHT as u32 ) as i32 }
	}


	pub fn save( size: WindowSize ) {
		let mut key = HKEY::default();
		if unsafe { RegCreateKeyW( HKEY_CURRENT_USER, w!( "Software\\EpStart" ), &mut key ) }.0 != 0 { return; }
		let width = size.width.max( 1 ) as u32;
		let height = size.height.max( 1 ) as u32;
		unsafe {
			let _ = RegSetValueExW( key, w!( "SettingsWidth" ), None, REG_DWORD, Some( &width.to_ne_bytes() ) );
			let _ = RegSetValueExW( key, w!( "SettingsHeight" ), None, REG_DWORD, Some( &height.to_ne_bytes() ) );
			let _ = RegCloseKey( key );
		}
	}
}


fn read_value( name: windows::core::PCWSTR ) -> Option< u32 > {
	let mut value = 0_u32;
	let mut size = size_of::< u32 >() as u32;
	let result = unsafe { RegGetValueW( HKEY_CURRENT_USER, w!( "Software\\EpStart" ), name, RRF_RT_REG_DWORD, None, Some( ( &mut value as *mut u32 ).cast::< c_void >() ), Some( &mut size ) ) };
	( result.0 == 0 ).then_some( value )
}
