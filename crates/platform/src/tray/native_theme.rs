//! ::  Project Path  ->  ep_start :: native_theme.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 16:14 周六


use std::ffi::c_void;
use std::mem::size_of;
use windows::Win32::Foundation::{ ERROR_SUCCESS, FreeLibrary };
use windows::Win32::System::LibraryLoader::{ GetProcAddress, LoadLibraryW };
use windows::Win32::System::Registry::{ HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW };
use windows::core::{ PCSTR, w };


type SetPreferredAppMode = unsafe extern "system" fn( i32 ) -> i32;
type FlushMenuThemes = unsafe extern "system" fn();


const FORCE_DARK_MODE: i32 = 2;
const FORCE_LIGHT_MODE: i32 = 3;


pub fn apply_native_menu_theme() -> bool {
	let dark = !apps_use_light_theme();
	let Ok( module ) = ( unsafe { LoadLibraryW( w!( "uxtheme.dll" ) ) } ) else { return dark; };
	unsafe {
		if let Some( address ) = GetProcAddress( module, ordinal( 135 ) ) {
			let set_preferred_app_mode: SetPreferredAppMode = std::mem::transmute( address );
			set_preferred_app_mode( if dark { FORCE_DARK_MODE } else { FORCE_LIGHT_MODE } );
		}
		if let Some( address ) = GetProcAddress( module, ordinal( 136 ) ) {
			let flush_menu_themes: FlushMenuThemes = std::mem::transmute( address );
			flush_menu_themes();
		}
		let _ = FreeLibrary( module );
	}
	dark
}


fn ordinal( value: usize ) -> PCSTR {
	PCSTR( value as *const c_void as *const u8 )
}


fn apps_use_light_theme() -> bool {
	let mut value = 1u32;
	let mut size = size_of::< u32 >() as u32;
	let result = unsafe { RegGetValueW( HKEY_CURRENT_USER, w!( "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize" ), w!( "AppsUseLightTheme" ), RRF_RT_REG_DWORD, None, Some( ( &mut value as *mut u32 ).cast::< c_void >() ), Some( &mut size ) ) };
	result == ERROR_SUCCESS && value != 0
}
