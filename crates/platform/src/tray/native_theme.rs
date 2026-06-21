//! ::  Project Path  ->  ep_start :: native_theme.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 16:14 周六


use std::ffi::c_void;
use windows::Win32::Foundation::FreeLibrary;
use windows::Win32::System::LibraryLoader::{ GetProcAddress, LoadLibraryW };
use windows::core::{ PCSTR, w };


type SetPreferredAppMode = unsafe extern "system" fn( i32 ) -> i32;
type FlushMenuThemes = unsafe extern "system" fn();


const ALLOW_DARK_MODE: i32 = 1;


pub fn apply_native_menu_theme() {
	let Ok( module ) = ( unsafe { LoadLibraryW( w!( "uxtheme.dll" ) ) } ) else { return; };
	unsafe {
		if let Some( address ) = GetProcAddress( module, ordinal( 135 ) ) {
			let set_preferred_app_mode: SetPreferredAppMode = std::mem::transmute( address );
			set_preferred_app_mode( ALLOW_DARK_MODE );
		}
		if let Some( address ) = GetProcAddress( module, ordinal( 136 ) ) {
			let flush_menu_themes: FlushMenuThemes = std::mem::transmute( address );
			flush_menu_themes();
		}
		let _ = FreeLibrary( module );
	}
}


fn ordinal( value: usize ) -> PCSTR {
	PCSTR( value as *const c_void as *const u8 )
}
