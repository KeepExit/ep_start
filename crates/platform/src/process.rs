//! ::  Project Path  ->  ep_start :: process.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 20:46 周六


use std::env;
use std::os::windows::ffi::OsStrExt;
use std::process::Command;
use std::thread;
use std::time::{ Duration, Instant };
use windows::Win32::Foundation::{ CloseHandle, WAIT_OBJECT_0 };
use windows::Win32::System::Threading::{ GetCurrentProcess, GetCurrentProcessId, INFINITE, OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, SetProcessWorkingSetSize, TerminateProcess, WaitForSingleObject };
use windows::Win32::UI::Shell::{ IsUserAnAdmin, ShellExecuteW };
use windows::Win32::UI::WindowsAndMessaging::{ FindWindowW, GetWindowThreadProcessId, SW_SHOWNORMAL };
use windows::core::{ PCWSTR, w };


const RESTART_HELPER_ARGUMENT: &str = "--ep-start-restart-shell-helper";


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


pub fn launch_shell_restart_helper() -> Result< (), String > {
	let executable = env::current_exe().map_err( |error| format!( "读取程序路径失败：{}", error ) )?;
	let working_directory = env::current_dir().map_err( |error| format!( "读取工作目录失败：{}", error ) )?;
	Command::new( executable ).arg( RESTART_HELPER_ARGUMENT ).arg( unsafe { GetCurrentProcessId() }.to_string() ).current_dir( working_directory ).spawn().map_err( |error| format!( "启动重启辅助进程失败：{}", error ) )?;
	Ok( () )
}


pub fn run_shell_restart_helper_if_requested() -> Result< bool, String > {
	let mut arguments = env::args();
	let _ = arguments.next();
	if arguments.next().as_deref() != Some( RESTART_HELPER_ARGUMENT ) { return Ok( false ); }
	let parent_id = arguments.next().ok_or_else( || "重启辅助进程缺少父进程 ID".to_string() )?.parse::< u32 >().map_err( |error| format!( "解析父进程 ID 失败：{}", error ) )?;
	let executable = env::current_exe().map_err( |error| format!( "读取程序路径失败：{}", error ) )?;
	let working_directory = env::current_dir().map_err( |error| format!( "读取工作目录失败：{}", error ) )?;
	wait_for_process_exit( parent_id );
	restart_explorer()?;
	Command::new( executable ).current_dir( working_directory ).spawn().map_err( |error| format!( "重新启动 EpStart 失败：{}", error ) )?;
	Ok( true )
}


pub fn trim_working_set() {
	unsafe { let _ = SetProcessWorkingSetSize( GetCurrentProcess(), usize::MAX, usize::MAX ); }
}


fn wait_for_process_exit( process_id: u32 ) {
	if let Ok( process ) = unsafe { OpenProcess( PROCESS_SYNCHRONIZE, false, process_id ) } {
		unsafe { let _ = WaitForSingleObject( process, INFINITE ); let _ = CloseHandle( process ); }
	} else { thread::sleep( Duration::from_millis( 500 ) ); }
}


fn restart_explorer() -> Result< (), String > {
	let taskbar = unsafe { FindWindowW( w!( "Shell_TrayWnd" ), PCWSTR::null() ) }.map_err( |error| format!( "找不到 Windows 资源管理器任务栏：{}", error ) )?;
	let mut explorer_id = 0;
	unsafe { GetWindowThreadProcessId( taskbar, Some( &mut explorer_id ) ); }
	if explorer_id == 0 { return Err( "无法读取 Windows 资源管理器进程 ID".to_string() ); }
	let explorer = unsafe { OpenProcess( PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, explorer_id ) }.map_err( |error| format!( "打开 Windows 资源管理器进程失败：{}", error ) )?;
	let terminate = unsafe { TerminateProcess( explorer, 0 ) };
	if let Err( error ) = terminate {
		unsafe { let _ = CloseHandle( explorer ); }
		return Err( format!( "终止 Windows 资源管理器失败：{}", error ) );
	}
	let wait = unsafe { WaitForSingleObject( explorer, 5000 ) };
	unsafe { let _ = CloseHandle( explorer ); }
	if wait != WAIT_OBJECT_0 { return Err( "等待 Windows 资源管理器退出超时".to_string() ); }
	if !wait_for_taskbar( explorer_id, Duration::from_millis( 1500 ) ) {
		let windows = env::var_os( "WINDIR" ).map( std::path::PathBuf::from ).ok_or_else( || "找不到 WINDIR 环境变量".to_string() )?;
		Command::new( windows.join( "explorer.exe" ) ).spawn().map_err( |error| format!( "启动 Windows 资源管理器失败：{}", error ) )?;
	}
	if !wait_for_taskbar( explorer_id, Duration::from_secs( 10 ) ) { return Err( "等待 Windows 资源管理器任务栏恢复超时".to_string() ); }
	Ok( () )
}


fn wait_for_taskbar( previous_process_id: u32, timeout: Duration ) -> bool {
	let started = Instant::now();
	while started.elapsed() < timeout {
		if let Ok( taskbar ) = unsafe { FindWindowW( w!( "Shell_TrayWnd" ), PCWSTR::null() ) } {
			let mut process_id = 0;
			unsafe { GetWindowThreadProcessId( taskbar, Some( &mut process_id ) ); }
			if process_id != 0 && process_id != previous_process_id { return true; }
		}
		thread::sleep( Duration::from_millis( 100 ) );
	}
	false
}
