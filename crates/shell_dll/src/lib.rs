//! ::  Project Path  ->  ep_start :: lib.rs :: shell_dll
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 22:42 周日

mod taskbar_click_hook;


use std::ffi::c_void;
use std::mem::size_of;
use std::sync::atomic::{ AtomicBool, AtomicPtr, AtomicU32, AtomicUsize, Ordering };
use windows::Win32::Foundation::{ CloseHandle, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WPARAM };
use windows::Win32::System::LibraryLoader::{ DisableThreadLibraryCalls, GetModuleFileNameW, GetModuleHandleW, GetProcAddress, LoadLibraryW };
use windows::Win32::System::Memory::{ PAGE_PROTECTION_FLAGS, PAGE_READWRITE, VirtualProtect };
use windows::Win32::System::SystemServices::{ DLL_PROCESS_ATTACH, IMAGE_DOS_SIGNATURE, IMAGE_NT_SIGNATURE };
use windows::Win32::System::Threading::{ OpenThread, QueueUserAPC, THREAD_SET_CONTEXT };
use windows::Win32::UI::Input::KeyboardAndMouse::{ MOD_WIN, UnregisterHotKey };
use windows::Win32::UI::WindowsAndMessaging::{ CallNextHookEx, EnumWindows, FindWindowW, GA_ROOT, GetAncestor, GetClassNameW, GetWindowThreadProcessId, HC_ACTION, MOUSEHOOKSTRUCT, MSG, PostMessageW, RegisterWindowMessageW, SC_TASKLIST, SEND_MESSAGE_TIMEOUT_FLAGS, SMTO_ABORTIFHUNG, SMTO_BLOCK, SendMessageTimeoutW, SetWindowsHookExW, WH_GETMESSAGE, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_NCPOINTERDOWN, WM_NULL, WM_POINTERDOWN };
use windows::core::{ BOOL, PCSTR, PCWSTR, w };


const SHELL_START_MESSAGE_NAME: PCWSTR = w!( "EpStart.Shell.StartKey.v1" );
const SHELL_START_BUTTON_STATE_MESSAGE_NAME: PCWSTR = w!( "EpStart.Shell.StartButtonState.v1" );
const SHELL_REGISTER_HOTKEY_ORDINAL: usize = 2671;
const START_ACTION_KEYBOARD: usize = 0;
const START_ACTION_BUTTON_CLICK: usize = 1;
const START_ACTION_TASKBAR_ACTIVATION: usize = 2;
const IMAGE_NT_OPTIONAL_HDR64_MAGIC: u16 = 0x020B;
const IMAGE_DIRECTORY_ENTRY_IMPORT: usize = 1;
const IMPORT_DESCRIPTOR_SIZE: usize = 20;
const IMPORT_DESCRIPTOR_NAME_OFFSET: usize = 12;
const IMPORT_DESCRIPTOR_FIRST_THUNK_OFFSET: usize = 16;
const OPTIONAL_HEADER_OFFSET: usize = 24;
const OPTIONAL_HEADER_SIZE_OF_IMAGE_OFFSET: usize = 56;
const OPTIONAL_HEADER_DATA_DIRECTORY_OFFSET: usize = 112;

static MODULE_INSTANCE: AtomicPtr< c_void > = AtomicPtr::new( std::ptr::null_mut() );
static INITIALIZED: AtomicBool = AtomicBool::new( false );
static PROGMAN_HOOK: AtomicPtr< c_void > = AtomicPtr::new( std::ptr::null_mut() );
static SHELL_START_MESSAGE: AtomicU32 = AtomicU32::new( 0 );
static SHELL_START_BUTTON_STATE_MESSAGE: AtomicU32 = AtomicU32::new( 0 );
static ORIGINAL_SHELL_REGISTER_HOTKEY: AtomicUsize = AtomicUsize::new( 0 );
type ShellRegisterHotKey = unsafe extern "system" fn( HWND, i32, u32, u32, HWND ) -> BOOL;


#[unsafe( no_mangle )]
pub unsafe extern "system" fn DllMain( instance: HINSTANCE, reason: u32, _reserved: *mut c_void ) -> BOOL {
	if reason == DLL_PROCESS_ATTACH {
		MODULE_INSTANCE.store( instance.0, Ordering::SeqCst );
		unsafe { let _ = DisableThreadLibraryCalls( HMODULE( instance.0 ) ); }
	}
	BOOL( 1 )
}


#[unsafe( no_mangle )]
pub unsafe extern "system" fn EpStartHookProc( code: i32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if code == HC_ACTION as i32 {
		if !INITIALIZED.load( Ordering::Acquire ) { initialize_explorer_bridge(); }
		if wparam.0 != 0 && lparam.0 != 0 {
			let message = unsafe { &mut *( lparam.0 as *mut MSG ) };
			if message.message == SHELL_START_BUTTON_STATE_MESSAGE.load( Ordering::SeqCst ) {
				taskbar_click_hook::set_experience_visible( message.wParam.0 != 0 );
				message.message = WM_NULL;
			} else if matches!( message.message, WM_LBUTTONDOWN | WM_LBUTTONDBLCLK | WM_POINTERDOWN | WM_NCPOINTERDOWN ) { taskbar_click_hook::note_pointer_down( message.hwnd, message.pt ); }
		}
	}
	unsafe { CallNextHookEx( None, code, wparam, lparam ) }
}


#[unsafe( no_mangle )]
pub unsafe extern "system" fn EpStartMouseHookProc( code: i32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if code == HC_ACTION as i32 && lparam.0 != 0 {
		if !INITIALIZED.load( Ordering::Acquire ) { initialize_explorer_bridge(); }
		let event = unsafe { &*( lparam.0 as *const MOUSEHOOKSTRUCT ) };
		match wparam.0 as u32 {
			WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => { taskbar_click_hook::note_pointer_down( event.hwnd, event.pt ); }
			WM_LBUTTONUP => {
				let start_handled = taskbar_click_hook::handle_pointer_up( event.hwnd, event.pt );
				if !start_handled && ( is_taskbar_activation_target( event.hwnd ) || taskbar_click_hook::is_taskbar_point( event.pt ) ) { if let Some( window ) = find_start_window() { post_start_action( window, START_ACTION_TASKBAR_ACTIVATION ); } }
			}
			_ => {}
		}
	}
	unsafe { CallNextHookEx( None, code, wparam, lparam ) }
}


unsafe extern "system" fn progman_hook_proc( code: i32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if code == HC_ACTION as i32 && wparam.0 != 0 && lparam.0 != 0 {
		let message = unsafe { &mut *( lparam.0 as *mut MSG ) };
		if message.message == windows::Win32::UI::WindowsAndMessaging::WM_SYSCOMMAND && message.wParam.0 as u32 & 0xFFF0 == SC_TASKLIST && route_start_action( START_ACTION_KEYBOARD ) { message.message = WM_NULL; }
	}
	unsafe { CallNextHookEx( None, code, wparam, lparam ) }
}


unsafe extern "system" fn shell_register_hotkey_hook( window: HWND, id: i32, modifiers: u32, key: u32, target: HWND ) -> BOOL {
	if modifiers == MOD_WIN.0 && key == 0 { return BOOL( 0 ); }
	let original = ORIGINAL_SHELL_REGISTER_HOTKEY.load( Ordering::SeqCst );
	if original == 0 { return BOOL( 0 ); }
	let original: ShellRegisterHotKey = unsafe { std::mem::transmute( original ) };
	unsafe { original( window, id, modifiers, key, target ) }
}


fn initialize_explorer_bridge() {
	if INITIALIZED.swap( true, Ordering::AcqRel ) { return; }
	let module = HMODULE( MODULE_INSTANCE.load( Ordering::SeqCst ) );
	if module.is_invalid() { return; }
	pin_module( module );
	SHELL_START_MESSAGE.store( unsafe { RegisterWindowMessageW( SHELL_START_MESSAGE_NAME ) }, Ordering::SeqCst );
	SHELL_START_BUTTON_STATE_MESSAGE.store( unsafe { RegisterWindowMessageW( SHELL_START_BUTTON_STATE_MESSAGE_NAME ) }, Ordering::SeqCst );
	install_progman_hook();
	let _ = taskbar_click_hook::install();
	if install_shell_hotkey_hook() { unregister_existing_win_hotkey(); }
}


fn pin_module( module: HMODULE ) {
	let mut path = [ 0u16; 32768 ];
	let length = unsafe { GetModuleFileNameW( Some( module ), &mut path ) } as usize;
	if length == 0 || length >= path.len() { return; }
	path[ length ] = 0;
	unsafe { let _ = LoadLibraryW( PCWSTR( path.as_ptr() ) ); }
}


fn install_progman_hook() {
	let Ok( progman ) = ( unsafe { FindWindowW( w!( "Progman" ), PCWSTR::null() ) } ) else { return; };
	let thread_id = unsafe { GetWindowThreadProcessId( progman, None ) };
	if thread_id == 0 { return; }
	if let Ok( hook ) = unsafe { SetWindowsHookExW( WH_GETMESSAGE, Some( progman_hook_proc ), None, thread_id ) } { PROGMAN_HOOK.store( hook.0, Ordering::SeqCst ); }
}


fn install_shell_hotkey_hook() -> bool {
	let Ok( user32 ) = ( unsafe { GetModuleHandleW( w!( "user32.dll" ) ) } ) else { return false; };
	let Some( original ) = ( unsafe { GetProcAddress( user32, PCSTR( SHELL_REGISTER_HOTKEY_ORDINAL as *const u8 ) ) } ) else { return false; };
	let original_address = original as usize;
	ORIGINAL_SHELL_REGISTER_HOTKEY.store( original_address, Ordering::SeqCst );
	let Ok( twinui ) = ( unsafe { GetModuleHandleW( w!( "twinui.dll" ) ) } ) else { return false; };
	unsafe { patch_import_address( twinui, original_address, shell_register_hotkey_hook as *const () as usize ) }
}


unsafe fn patch_import_address( module: HMODULE, original: usize, replacement: usize ) -> bool {
	let base = module.0 as *mut u8;
	if base.is_null() || unsafe { read_u16( base, 0 ) } != IMAGE_DOS_SIGNATURE { return false; }
	let nt_offset = unsafe { read_u32( base, 0x3C ) } as usize;
	if unsafe { read_u32( base, nt_offset ) } != IMAGE_NT_SIGNATURE { return false; }
	let optional = nt_offset + OPTIONAL_HEADER_OFFSET;
	if unsafe { read_u16( base, optional ) } != IMAGE_NT_OPTIONAL_HDR64_MAGIC { return false; }
	let image_size = unsafe { read_u32( base, optional + OPTIONAL_HEADER_SIZE_OF_IMAGE_OFFSET ) } as usize;
	let import_directory = optional + OPTIONAL_HEADER_DATA_DIRECTORY_OFFSET + IMAGE_DIRECTORY_ENTRY_IMPORT * 8;
	let import_rva = unsafe { read_u32( base, import_directory ) } as usize;
	let import_size = unsafe { read_u32( base, import_directory + 4 ) } as usize;
	if import_rva == 0 || import_rva >= image_size { return false; }
	let descriptor_count = import_size / IMPORT_DESCRIPTOR_SIZE;
	for index in 0..descriptor_count {
		let descriptor = import_rva + index * IMPORT_DESCRIPTOR_SIZE;
		let name_rva = unsafe { read_u32( base, descriptor + IMPORT_DESCRIPTOR_NAME_OFFSET ) } as usize;
		let first_thunk = unsafe { read_u32( base, descriptor + IMPORT_DESCRIPTOR_FIRST_THUNK_OFFSET ) } as usize;
		if name_rva == 0 && first_thunk == 0 { break; }
		if name_rva >= image_size || first_thunk >= image_size || !unsafe { import_name_matches( base, image_size, name_rva, b"user32.dll" ) } { continue; }
		let mut thunk_offset = first_thunk;
		while thunk_offset + size_of::< usize >() <= image_size {
			let thunk = unsafe { base.add( thunk_offset ).cast::< AtomicUsize >() };
			let value = unsafe { ( *thunk ).load( Ordering::SeqCst ) };
			if value == 0 { break; }
			if value == original { return unsafe { replace_thunk( thunk, replacement ) }; }
			thunk_offset += size_of::< usize >();
		}
	}
	false
}


unsafe fn replace_thunk( thunk: *mut AtomicUsize, replacement: usize ) -> bool {
	let mut previous = PAGE_PROTECTION_FLAGS::default();
	if unsafe { VirtualProtect( thunk.cast(), size_of::< usize >(), PAGE_READWRITE, &mut previous ) }.is_err() { return false; }
	unsafe { ( *thunk ).store( replacement, Ordering::SeqCst ); }
	let mut ignored = PAGE_PROTECTION_FLAGS::default();
	unsafe { let _ = VirtualProtect( thunk.cast(), size_of::< usize >(), previous, &mut ignored ); }
	true
}


unsafe fn import_name_matches( base: *const u8, image_size: usize, name_rva: usize, expected: &[ u8 ] ) -> bool {
	if name_rva + expected.len() + 1 > image_size { return false; }
	let name = unsafe { std::slice::from_raw_parts( base.add( name_rva ), expected.len() + 1 ) };
	name[ expected.len() ] == 0 && name[ ..expected.len() ].eq_ignore_ascii_case( expected )
}


unsafe fn read_u16( base: *const u8, offset: usize ) -> u16 {
	unsafe { std::ptr::read_unaligned( base.add( offset ).cast::< u16 >() ) }
}


unsafe fn read_u32( base: *const u8, offset: usize ) -> u32 {
	unsafe { std::ptr::read_unaligned( base.add( offset ).cast::< u32 >() ) }
}


fn unregister_existing_win_hotkey() {
	let Ok( shell_window ) = ( unsafe { FindWindowW( w!( "ApplicationManager_ImmersiveShellWindow" ), PCWSTR::null() ) } ) else { return; };
	let thread_id = unsafe { GetWindowThreadProcessId( shell_window, None ) };
	if thread_id == 0 { return; }
	let Ok( thread ) = ( unsafe { OpenThread( THREAD_SET_CONTEXT, false, thread_id ) } ) else { return; };
	unsafe { let _ = QueueUserAPC( Some( unregister_win_hotkey_apc ), thread, 0 ); let _ = CloseHandle( thread ); }
}


unsafe extern "system" fn unregister_win_hotkey_apc( _parameter: usize ) {
	unsafe { let _ = UnregisterHotKey( None, 1 ); }
}


fn find_start_window() -> Option< HWND > {
	let mut window = HWND::default();
	unsafe { let _ = EnumWindows( Some( find_start_window_callback ), LPARAM( ( &mut window as *mut HWND ) as isize ) ); }
	( !window.is_invalid() ).then_some( window )
}


unsafe extern "system" fn find_start_window_callback( hwnd: HWND, lparam: LPARAM ) -> BOOL {
	let mut class_name = [ 0u16; 64 ];
	let length = unsafe { GetClassNameW( hwnd, &mut class_name ) };
	if length <= 0 { return BOOL( 1 ); }
	let class_name = String::from_utf16_lossy( &class_name[ ..length as usize ] );
	if class_name != "Windows.UI.EpStartWindow" { return BOOL( 1 ); }
	unsafe { *( lparam.0 as *mut HWND ) = hwnd; }
	BOOL( 0 )
}


fn is_taskbar_activation_target( target: HWND ) -> bool {
	if target.is_invalid() { return false; }
	let root = unsafe { GetAncestor( target, GA_ROOT ) };
	if root.is_invalid() { return false; }
	matches!( window_class_name( root ).as_str(), "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "TaskListThumbnailWnd" | "TaskListThumbnailWndXaml" | "XamlExplorerHostIslandWindow" )
}


fn window_class_name( window: HWND ) -> String {
	let mut class_name = [ 0u16; 128 ];
	let length = unsafe { GetClassNameW( window, &mut class_name ) };
	if length <= 0 { return String::new(); }
	String::from_utf16_lossy( &class_name[ ..length as usize ] )
}


fn post_start_action( window: HWND, source: usize ) {
	let message = SHELL_START_MESSAGE.load( Ordering::SeqCst );
	if message == 0 { return; }
	unsafe { let _ = PostMessageW( Some( window ), message, WPARAM( source ), LPARAM( 0 ) ); }
}


fn route_start_action( source: usize ) -> bool {
	let message = SHELL_START_MESSAGE.load( Ordering::SeqCst );
	if message == 0 { return false; }
	let Some( window ) = find_start_window() else { return false; };
	let mut handled = 0usize;
	let flags = SEND_MESSAGE_TIMEOUT_FLAGS( SMTO_BLOCK.0 | SMTO_ABORTIFHUNG.0 );
	let sent = unsafe { SendMessageTimeoutW( window, message, WPARAM( source ), LPARAM( 0 ), flags, 100, Some( &mut handled ) ) };
	sent.0 != 0 && handled == 1
}
