//! ::  Project Path  ->  ep_start :: lib.rs :: injector
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/22 01:20 周一


use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::Win32::Foundation::{ CloseHandle, FreeLibrary, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WPARAM };
use windows::Win32::System::LibraryLoader::{ GetProcAddress, LoadLibraryW };
use windows::Win32::System::Threading::{ OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW };
use windows::Win32::UI::WindowsAndMessaging::{ EnumWindows, FindWindowW, GetClassNameW, GetWindowThreadProcessId, HHOOK, HOOKPROC, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx, WH_GETMESSAGE, WH_MOUSE, WM_NULL };
use windows::core::{ PCSTR, PCWSTR, PWSTR, w };


const HOOK_EXPORT_NAME: PCSTR = PCSTR( b"EpStartHookProc\0".as_ptr() );
const MOUSE_HOOK_EXPORT_NAME: PCSTR = PCSTR( b"EpStartMouseHookProc\0".as_ptr() );


pub struct ShellInjection {
	message_hooks: Vec< HHOOK >,
	mouse_hooks: Vec< HHOOK >,
	module: HMODULE,
}


struct ExplorerMouseThreads {
	process_id: u32,
	thread_ids: Vec< u32 >,
}


impl ShellInjection {
	pub fn install( dll_path: &Path ) -> Result< Self, String > {
		if !dll_path.is_file() { return Err( format!( "Explorer 桥接 DLL 不存在：{}", dll_path.display() ) ); }
		let taskbar = unsafe { FindWindowW( w!( "Shell_TrayWnd" ), PCWSTR::null() ) }.map_err( |error| format!( "查找主任务栏失败：{}", error ) )?;
		let ( thread_id, process_id ) = explorer_thread( taskbar )?;
		let path = wide_null( dll_path.as_os_str() );
		let module = unsafe { LoadLibraryW( PCWSTR( path.as_ptr() ) ) }.map_err( |error| format!( "加载 Explorer 桥接 DLL 失败：{}", error ) )?;
		let procedure = unsafe { GetProcAddress( module, HOOK_EXPORT_NAME ) };
		let Some( procedure ) = procedure else {
			unsafe { let _ = FreeLibrary( module ); }
			return Err( "Explorer 桥接 DLL 缺少 EpStartHookProc 导出函数".to_string() );
		};
		let mouse_procedure = unsafe { GetProcAddress( module, MOUSE_HOOK_EXPORT_NAME ) };
		let Some( mouse_procedure ) = mouse_procedure else {
			unsafe { let _ = FreeLibrary( module ); }
			return Err( "Explorer 桥接 DLL 缺少 EpStartMouseHookProc 导出函数".to_string() );
		};
		let message_procedure: HOOKPROC = Some( unsafe { std::mem::transmute::< unsafe extern "system" fn() -> isize, unsafe extern "system" fn( i32, WPARAM, LPARAM ) -> LRESULT >( procedure ) } );
		let mouse_procedure: HOOKPROC = Some( unsafe { std::mem::transmute::< unsafe extern "system" fn() -> isize, unsafe extern "system" fn( i32, WPARAM, LPARAM ) -> LRESULT >( mouse_procedure ) } );
		let explorer_threads = explorer_mouse_thread_ids( thread_id, process_id );
		let mut message_hooks = Vec::new();
		let mut mouse_hooks = Vec::new();
		for explorer_thread_id in explorer_threads {
			match unsafe { SetWindowsHookExW( WH_GETMESSAGE, message_procedure, Some( HINSTANCE( module.0 ) ), explorer_thread_id ) } {
				Ok( hook ) => message_hooks.push( hook ),
				Err( error ) => {
					unsafe { for hook in mouse_hooks.drain( .. ) { let _ = UnhookWindowsHookEx( hook ); } for hook in message_hooks.drain( .. ) { let _ = UnhookWindowsHookEx( hook ); } let _ = FreeLibrary( module ); }
					return Err( format!( "安装 Explorer 线程 {} 的消息钩子失败：{}", explorer_thread_id, error ) );
				}
			}
			match unsafe { SetWindowsHookExW( WH_MOUSE, mouse_procedure, Some( HINSTANCE( module.0 ) ), explorer_thread_id ) } {
				Ok( hook ) => mouse_hooks.push( hook ),
				Err( error ) => {
					unsafe { for hook in mouse_hooks.drain( .. ) { let _ = UnhookWindowsHookEx( hook ); } for hook in message_hooks.drain( .. ) { let _ = UnhookWindowsHookEx( hook ); } let _ = FreeLibrary( module ); }
					return Err( format!( "安装 Explorer 线程 {} 的鼠标钩子失败：{}", explorer_thread_id, error ) );
				}
			}
		}
		if let Err( error ) = unsafe { PostMessageW( Some( taskbar ), WM_NULL, WPARAM( 0 ), LPARAM( 0 ) ) } {
			unsafe { for hook in mouse_hooks.drain( .. ) { let _ = UnhookWindowsHookEx( hook ); } for hook in message_hooks.drain( .. ) { let _ = UnhookWindowsHookEx( hook ); } let _ = FreeLibrary( module ); }
			return Err( format!( "激活 Explorer 消息钩子失败：{}", error ) );
		}
		Ok( Self { message_hooks, mouse_hooks, module } )
	}
}


impl Drop for ShellInjection {
	fn drop( &mut self ) {
		unsafe {
			for hook in self.mouse_hooks.drain( .. ) { let _ = UnhookWindowsHookEx( hook ); }
			for hook in self.message_hooks.drain( .. ) { let _ = UnhookWindowsHookEx( hook ); }
			let _ = FreeLibrary( self.module );
		}
	}
}


fn explorer_thread( taskbar: HWND ) -> Result< ( u32, u32 ), String > {
	let mut process_id = 0;
	let thread_id = unsafe { GetWindowThreadProcessId( taskbar, Some( &mut process_id ) ) };
	if thread_id == 0 || process_id == 0 { return Err( "无法读取任务栏所属线程".to_string() ); }
	let process = unsafe { OpenProcess( PROCESS_QUERY_LIMITED_INFORMATION, false, process_id ) }.map_err( |error| format!( "打开任务栏进程失败：{}", error ) )?;
	let mut path = [ 0u16; 32768 ];
	let mut length = path.len() as u32;
	let query = unsafe { QueryFullProcessImageNameW( process, PROCESS_NAME_FORMAT( 0 ), PWSTR( path.as_mut_ptr() ), &mut length ) };
	unsafe { let _ = CloseHandle( process ); }
	query.map_err( |error| format!( "读取任务栏进程路径失败：{}", error ) )?;
	let image = String::from_utf16_lossy( &path[ ..length as usize ] );
	if !image.rsplit( '\\' ).next().is_some_and( |name| name.eq_ignore_ascii_case( "explorer.exe" ) ) { return Err( format!( "Shell_TrayWnd 不属于 explorer.exe：{}", image ) ); }
	Ok( ( thread_id, process_id ) )
}


fn explorer_mouse_thread_ids( taskbar_thread_id: u32, process_id: u32 ) -> Vec< u32 > {
	let mut search = ExplorerMouseThreads { process_id, thread_ids: vec![ taskbar_thread_id ] };
	unsafe { let _ = EnumWindows( Some( collect_explorer_mouse_thread ), LPARAM( ( &mut search as *mut ExplorerMouseThreads ) as isize ) ); }
	search.thread_ids.sort_unstable();
	search.thread_ids.dedup();
	search.thread_ids
}


unsafe extern "system" fn collect_explorer_mouse_thread( hwnd: HWND, lparam: LPARAM ) -> windows::core::BOOL {
	let search = unsafe { &mut *( lparam.0 as *mut ExplorerMouseThreads ) };
	let mut owner_process_id = 0;
	let thread_id = unsafe { GetWindowThreadProcessId( hwnd, Some( &mut owner_process_id ) ) };
	if owner_process_id != search.process_id || thread_id == 0 { return true.into(); }
	let mut class_name = [ 0u16; 128 ];
	let length = unsafe { GetClassNameW( hwnd, &mut class_name ) };
	if length <= 0 { return true.into(); }
	let class_name = String::from_utf16_lossy( &class_name[ ..length as usize ] );
	if matches!( class_name.as_str(), "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "XamlExplorerHostIslandWindow" | "TaskListThumbnailWnd" | "TaskListThumbnailWndXaml" ) { search.thread_ids.push( thread_id ); }
	true.into()
}


fn wide_null( value: &OsStr ) -> Vec< u16 > {
	value.encode_wide().chain( [ 0 ] ).collect()
}
