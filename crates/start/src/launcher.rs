//! ::  Project Path  ->  ep_start :: launcher.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 02:20 周六


use crate::config::Tile;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{ PCWSTR, w };


pub struct ProgramLauncher;


impl ProgramLauncher {
	pub fn launch( hwnd: HWND, tile: &Tile ) -> Result< (), String > {
		let target = wide_string( tile.target.as_ref() );
		let arguments = wide_string( tile.arguments.as_ref() );
		let directory = tile.working_directory().map( |path| path.as_os_str().encode_wide().chain( [ 0 ] ).collect::< Vec< u16 > >() );
		let argument_pointer = if tile.arguments.is_empty() { PCWSTR::null() } else { PCWSTR( arguments.as_ptr() ) };
		let directory_pointer = directory.as_ref().map_or( PCWSTR::null(), |value| PCWSTR( value.as_ptr() ) );
		let result = unsafe { ShellExecuteW( Some( hwnd ), w!( "open" ), PCWSTR( target.as_ptr() ), argument_pointer, directory_pointer, SW_SHOWNORMAL ) };
		if result.0 as isize <= 32 { return Err( format!( "无法启动「{}」，ShellExecuteW 返回 {}", tile.title, result.0 as isize ) ); }
		Ok( () )
	}
}


fn wide_string( value: &str ) -> Vec< u16 > {
	value.encode_utf16().chain( [ 0 ] ).collect()
}
