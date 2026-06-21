//! ::  Project Path  ->  ep_start :: process.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 20:46 周六


use std::env;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::UI::Shell::{ IsUserAnAdmin, ShellExecuteW };
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{ PCWSTR, w };
use windows::Win32::System::Threading::{ GetCurrentProcess, SetProcessWorkingSetSize };


pub fn ensure_elevated() -> Result< bool, String > {
	if unsafe { IsUserAnAdmin() }.as_bool() { return Ok( true ); }
	let executable = env::current_exe().map_err( |error| format!( "读取程序路径失败：{}", error ) )?;
	let working_directory = env::current_dir().map_err( |error| format!( "读取工作目录失败：{}", error ) )?;
	let executable: Vec< u16 > = executable.as_os_str().encode_wide().chain( [ 0 ] ).collect();
	let working_directory: Vec< u16 > = working_directory.as_os_str().encode_wide().chain( [ 0 ] ).collect();
	let result = unsafe { ShellExecuteW( None, w!( "runas" ), PCWSTR( executable.as_ptr() ), PCWSTR::null(), PCWSTR( working_directory.as_ptr() ), SW_SHOWNORMAL ) };
	if result.0 as isize <= 32 { return Err( format!( "请求管理员权限失败，ShellExecuteW 返回 {}", result.0 as isize ) ); }
	Ok( false )
}


pub fn trim_working_set() {
	unsafe { let _ = SetProcessWorkingSetSize( GetCurrentProcess(), usize::MAX, usize::MAX ); }
}
