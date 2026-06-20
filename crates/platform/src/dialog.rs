//! ::  Project Path  ->  ep_start :: dialog.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 03:17 周六


use windows::Win32::UI::WindowsAndMessaging::{ MB_ICONERROR, MB_OK, MessageBoxW };
use windows::core::PCWSTR;


pub fn show_error_dialog( title: &str, message: &str ) {
	let title = wide_string( title );
	let message = wide_string( message );
	unsafe { MessageBoxW( None, PCWSTR( message.as_ptr() ), PCWSTR( title.as_ptr() ), MB_OK | MB_ICONERROR ); }
}


fn wide_string( value: &str ) -> Vec< u16 > {
	value.encode_utf16().chain( [ 0 ] ).collect()
}
