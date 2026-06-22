//! ::  Project Path  ->  ep_start :: input.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 03:17 周六

use std::ffi::c_void;
use std::mem::size_of;
use std::sync::mpsc::{ SyncSender, sync_channel };
use std::sync::atomic::{ AtomicBool, AtomicPtr, AtomicU8, AtomicU32, Ordering };
use std::thread::{ self, JoinHandle };
use windows::Win32::Foundation::{ CloseHandle, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{GetCurrentThreadId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT};
use windows::Win32::UI::Accessibility::{ HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent };
use windows::Win32::UI::Input::{ GetRawInputData, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWKEYBOARD, RIDEV_INPUTSINK, RIDEV_REMOVE, RID_INPUT, RIM_TYPEKEYBOARD, RegisterRawInputDevices };
use windows::Win32::UI::Input::KeyboardAndMouse::{ INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_ESCAPE, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_TAB };
use windows::Win32::UI::WindowsAndMessaging::{CallNextHookEx, DispatchMessageW, GetMessageTime, GetMessageW, GetWindowThreadProcessId, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, LLKHF_INJECTED, MSG, PM_NOREMOVE, PeekMessageW, PostMessageW, PostThreadMessageW, RI_KEY_BREAK, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT};
use windows::core::PWSTR;


const LEFT_SHIFT_DOWN: u8 = 1 << 0;
const RIGHT_SHIFT_DOWN: u8 = 1 << 1;
const LEFT_WIN_DOWN: u8 = 1 << 2;
const RIGHT_WIN_DOWN: u8 = 1 << 3;
const LEFT_ALT_DOWN: u8 = 1 << 4;
const RIGHT_ALT_DOWN: u8 = 1 << 5;
const SHIFT_DOWN: u8 = LEFT_SHIFT_DOWN | RIGHT_SHIFT_DOWN;
const WIN_DOWN: u8 = LEFT_WIN_DOWN | RIGHT_WIN_DOWN;
const ALT_DOWN: u8 = LEFT_ALT_DOWN | RIGHT_ALT_DOWN;
const NATIVE_START_EVENT_GAP_MS: u32 = 180;

static TARGET_WINDOW: AtomicPtr< c_void > = AtomicPtr::new( std::ptr::null_mut() );
static TOGGLE_MESSAGE: AtomicU32 = AtomicU32::new( 0 );
static DISMISS_MESSAGE: AtomicU32 = AtomicU32::new( 0 );
static ALT_TAB_MESSAGE: AtomicU32 = AtomicU32::new( 0 );
static SURFACE_VISIBLE: AtomicBool = AtomicBool::new( false );
static MODIFIER_STATE: AtomicU8 = AtomicU8::new( 0 );
static RAW_MODIFIER_STATE: AtomicU8 = AtomicU8::new( 0 );
static LAST_TOGGLE_EVENT_TIME: AtomicU32 = AtomicU32::new( 0 );
static LAST_NATIVE_START_EVENT_TIME: AtomicU32 = AtomicU32::new( 0 );
static ESCAPE_CAPTURED: AtomicBool = AtomicBool::new( false );
static SHORTCUT_MODE: AtomicU8 = AtomicU8::new( GlobalStartShortcut::WinShift as u8 );
static WIN_SEQUENCE_USED: AtomicBool = AtomicBool::new( false );
static WIN_SEQUENCE_HANDLED: AtomicBool = AtomicBool::new( false );
static RAW_WIN_SEQUENCE_USED: AtomicBool = AtomicBool::new( false );
static RAW_WIN_SEQUENCE_HANDLED: AtomicBool = AtomicBool::new( false );
static ALT_TAB_SEQUENCE_ACTIVE: AtomicBool = AtomicBool::new( false );
static ALT_TAB_ESCAPE_CANCELLED: AtomicBool = AtomicBool::new( false );


pub struct GlobalInputManager {
	hook_thread_id: u32,
	hook_thread: Option< JoinHandle< () > >,
}


pub struct GlobalInputBinding {
	hwnd: HWND,
}


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub enum GlobalInputAction {
	Toggle,
	Dismiss,
}


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
#[repr( usize )]
pub enum GlobalAltTabEvent {
	Started = 1,
	Committed = 2,
	Cancelled = 3,
}


impl GlobalAltTabEvent {
	pub fn from_message_parameter( value: usize ) -> Option< Self > {
		match value {
			1 => Some( Self::Started ),
			2 => Some( Self::Committed ),
			3 => Some( Self::Cancelled ),
			_ => None,
		}
	}
}


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
#[repr( u8 )]
pub enum GlobalStartShortcut {
	WinShift = 0,
	Win = 1,
}


impl GlobalInputManager {
	pub fn new() -> Result< Self, String > {
		let ( ready_sender, ready_receiver ) = sync_channel( 1 );
		let hook_thread = thread::Builder::new().name( "ep-start-input-hook".to_string() ).spawn( move || run_hook_thread( ready_sender ) ).map_err( |error| format!( "创建全局输入线程失败：{}", error ) )?;
		match ready_receiver.recv().map_err( |error| format!( "等待全局输入线程失败：{}", error ) )? {
			Ok( hook_thread_id ) => Ok( Self { hook_thread_id, hook_thread: Some( hook_thread ) } ),
			Err( error ) => { let _ = hook_thread.join(); Err( error ) }
		}
	}


	pub fn bind_start_surface( &self, hwnd: HWND, toggle_message: u32, dismiss_message: u32, alt_tab_message: u32 ) -> Result< GlobalInputBinding, String > {
		if hwnd.is_invalid() { return Err( "无法为无效窗口绑定 Start 快捷键".to_string() ); }
		reset_input_state();
		TOGGLE_MESSAGE.store( toggle_message, Ordering::SeqCst );
		DISMISS_MESSAGE.store( dismiss_message, Ordering::SeqCst );
		ALT_TAB_MESSAGE.store( alt_tab_message, Ordering::SeqCst );
		TARGET_WINDOW.compare_exchange( std::ptr::null_mut(), hwnd.0, Ordering::SeqCst, Ordering::SeqCst ).map_err( |_| "Start 全局快捷键已经绑定".to_string() )?;
		if let Err( error ) = register_raw_keyboard( hwnd ) {
			TARGET_WINDOW.store( std::ptr::null_mut(), Ordering::SeqCst );
			return Err( error );
		}
		Ok( GlobalInputBinding { hwnd } )
	}
}


impl Drop for GlobalInputManager {
	fn drop( &mut self ) {
		unsafe { let _ = PostThreadMessageW( self.hook_thread_id, WM_QUIT, WPARAM( 0 ), LPARAM( 0 ) ); }
		if let Some( hook_thread ) = self.hook_thread.take() { let _ = hook_thread.join(); }
		TARGET_WINDOW.store( std::ptr::null_mut(), Ordering::SeqCst );
		reset_input_state();
	}
}


fn run_hook_thread( ready: SyncSender< Result< u32, String > > ) {
	let module = match unsafe { GetModuleHandleW( None ) } {
		Ok( module ) => module,
		Err( error ) => { let _ = ready.send( Err( format!( "读取程序模块句柄失败：{}", error ) ) ); return; }
	};
	let keyboard_hook = match unsafe { SetWindowsHookExW( WH_KEYBOARD_LL, Some( keyboard_hook ), Some( HINSTANCE( module.0 ) ), 0 ) } {
		Ok( hook ) => hook,
		Err( error ) => { let _ = ready.send( Err( format!( "安装全局输入监听失败：{}", error ) ) ); return; }
	};
	let shell_hook = unsafe { SetWinEventHook( EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some( shell_foreground_event ), 0, 0, WINEVENT_OUTOFCONTEXT ) };
	let mut message = MSG::default();
	unsafe { let _ = PeekMessageW( &mut message, None, 0, 0, PM_NOREMOVE ); }
	if ready.send( Ok( unsafe { GetCurrentThreadId() } ) ).is_err() {
		unsafe {
			let _ = UnhookWindowsHookEx( keyboard_hook );
			if !shell_hook.is_invalid() { let _ = UnhookWinEvent( shell_hook ); }
		}
		return;
	}
	loop {
		let result = unsafe { GetMessageW( &mut message, None, 0, 0 ) };
		if result.0 <= 0 { break; }
		unsafe { let _ = TranslateMessage( &message ); DispatchMessageW( &message ); }
	}
	unsafe {
		let _ = UnhookWindowsHookEx( keyboard_hook );
		if !shell_hook.is_invalid() { let _ = UnhookWinEvent( shell_hook ); }
	}
}


impl GlobalInputBinding {
	pub fn set_shortcut( &self, shortcut: GlobalStartShortcut ) {
		SHORTCUT_MODE.store( shortcut as u8, Ordering::SeqCst );
		reset_shortcut_state();
	}


	pub fn set_surface_visible( &self, visible: bool ) {
		SURFACE_VISIBLE.store( visible, Ordering::SeqCst );
		if !visible { ALT_TAB_SEQUENCE_ACTIVE.store( false, Ordering::SeqCst ); }
	}


	pub fn raw_input_action( &self, lparam: LPARAM ) -> Option< GlobalInputAction > {
		let keyboard = read_raw_keyboard( lparam )?;
		let key_down = keyboard.Flags as u32 & RI_KEY_BREAK == 0;
		if keyboard.VKey == VK_ESCAPE.0 && key_down && ALT_TAB_ESCAPE_CANCELLED.swap( false, Ordering::SeqCst ) { return None; }
		if keyboard.VKey == VK_ESCAPE.0 && key_down && SURFACE_VISIBLE.load( Ordering::SeqCst ) { return Some( GlobalInputAction::Dismiss ); }
		let modifier = raw_modifier_mask( &keyboard );
		let previous = RAW_MODIFIER_STATE.load( Ordering::SeqCst );
		let current = if key_down { previous | modifier } else { previous & !modifier };
		RAW_MODIFIER_STATE.store( current, Ordering::SeqCst );
		handle_raw_shortcut_event( keyboard.VKey, modifier, previous, current, key_down, unsafe { GetMessageTime() } as u32 )
	}
}


impl Drop for GlobalInputBinding {
	fn drop( &mut self ) {
		unregister_raw_keyboard();
		let _ = TARGET_WINDOW.compare_exchange( self.hwnd.0, std::ptr::null_mut(), Ordering::SeqCst, Ordering::SeqCst );
		TOGGLE_MESSAGE.store( 0, Ordering::SeqCst );
		DISMISS_MESSAGE.store( 0, Ordering::SeqCst );
		ALT_TAB_MESSAGE.store( 0, Ordering::SeqCst );
		reset_input_state();
	}
}


unsafe extern "system" fn keyboard_hook( code: i32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if code == HC_ACTION as i32 {
		let event = unsafe { &*( lparam.0 as *const KBDLLHOOKSTRUCT ) };
		if event.flags.0 & LLKHF_INJECTED.0 != 0 { return unsafe { CallNextHookEx( None, code, wparam, lparam ) }; }

		let message = wparam.0 as u32;
		let key_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
		let key_up = message == WM_KEYUP || message == WM_SYSKEYUP;

		if key_down && event.vkCode as u16 == VK_TAB.0 && event.flags.0 & LLKHF_ALTDOWN.0 != 0 { start_alt_tab_sequence(); }
		let cancelling_alt_tab = event.vkCode as u16 == VK_ESCAPE.0 && ALT_TAB_SEQUENCE_ACTIVE.load( Ordering::SeqCst );
		if cancelling_alt_tab && key_down { cancel_alt_tab_sequence(); }
		if !cancelling_alt_tab && event.vkCode as u16 == VK_ESCAPE.0 && handle_escape( key_down, key_up ) { return LRESULT( 1 ); }
		if ( key_down || key_up ) && handle_shortcut_event( event.vkCode as u16, event.scanCode, key_down, event.time ) { return LRESULT( 1 ); }
	}

	unsafe { CallNextHookEx( None, code, wparam, lparam ) }
}


unsafe extern "system" fn shell_foreground_event( _hook: HWINEVENTHOOK, event: u32, foreground: HWND, _object_id: i32, _child_id: i32, _event_thread: u32, event_time: u32 ) {
	if event != EVENT_SYSTEM_FOREGROUND { return; }
	if TARGET_WINDOW.load( Ordering::SeqCst ).is_null() || TOGGLE_MESSAGE.load( Ordering::SeqCst ) == 0 { return; }
	if current_shortcut() != GlobalStartShortcut::Win { return; }
	if !is_native_start_menu_window( foreground ) { return; }
	if !claim_native_start_event( event_time ) { return; }

	dismiss_native_start_menu();

	if SURFACE_VISIBLE.load( Ordering::SeqCst ) { return; }

	post_message( TOGGLE_MESSAGE.load( Ordering::SeqCst ) );
}


fn handle_escape( key_down: bool, key_up: bool ) -> bool {
	if key_down && SURFACE_VISIBLE.load( Ordering::SeqCst ) {
		if !ESCAPE_CAPTURED.swap( true, Ordering::SeqCst ) { post_message( DISMISS_MESSAGE.load( Ordering::SeqCst ) ); }
		return true;
	}
	if key_up && ESCAPE_CAPTURED.swap( false, Ordering::SeqCst ) { return true; }
	false
}


fn handle_shortcut_event( key: u16, scan_code: u32, key_down: bool, event_time: u32 ) -> bool {
	let modifier = modifier_mask( key as u32, scan_code );
	let previous = MODIFIER_STATE.load( Ordering::SeqCst );
	let current = if key_down { previous | modifier } else { previous & !modifier };
	MODIFIER_STATE.store( current, Ordering::SeqCst );
	handle_alt_tab_event( key, modifier, current, key_down );
	let win_key = modifier & WIN_DOWN != 0;
	let shortcut = current_shortcut();

	if shortcut == GlobalStartShortcut::Win { return false; }

	if key_down && win_key && previous & WIN_DOWN == 0 {
		WIN_SEQUENCE_USED.store( false, Ordering::SeqCst );
		WIN_SEQUENCE_HANDLED.store( false, Ordering::SeqCst );
	}

	if key_down && !win_key && previous & WIN_DOWN != 0 && modifier & SHIFT_DOWN == 0 {
		WIN_SEQUENCE_USED.store( true, Ordering::SeqCst );
	}

	if !chord_complete( previous ) && chord_complete( current ) {
		WIN_SEQUENCE_HANDLED.store( true, Ordering::SeqCst );
		if claim_toggle_event( event_time ) { post_message( TOGGLE_MESSAGE.load( Ordering::SeqCst ) ); }
	}

	if !key_down && win_key && current & WIN_DOWN == 0 {
		let used = WIN_SEQUENCE_USED.swap( false, Ordering::SeqCst );
		let handled = WIN_SEQUENCE_HANDLED.swap( false, Ordering::SeqCst );
		if !handled && !used && SURFACE_VISIBLE.load( Ordering::SeqCst ) { post_message( DISMISS_MESSAGE.load( Ordering::SeqCst ) ); }
	}

	false
}


fn handle_alt_tab_event( key: u16, modifier: u8, current: u8, key_down: bool ) {
	if key_down && key == VK_TAB.0 && current & ALT_DOWN != 0 && SURFACE_VISIBLE.load( Ordering::SeqCst ) {
		start_alt_tab_sequence();
		return;
	}
	if !key_down && modifier & ALT_DOWN != 0 && current & ALT_DOWN == 0 && ALT_TAB_SEQUENCE_ACTIVE.swap( false, Ordering::SeqCst ) { post_alt_tab_event( GlobalAltTabEvent::Committed ); }
}


fn start_alt_tab_sequence() {
	if SURFACE_VISIBLE.load( Ordering::SeqCst ) && !ALT_TAB_SEQUENCE_ACTIVE.swap( true, Ordering::SeqCst ) { post_alt_tab_event( GlobalAltTabEvent::Started ); }
}


fn cancel_alt_tab_sequence() {
	if ALT_TAB_SEQUENCE_ACTIVE.swap( false, Ordering::SeqCst ) {
		ALT_TAB_ESCAPE_CANCELLED.store( true, Ordering::SeqCst );
		post_alt_tab_event( GlobalAltTabEvent::Cancelled );
	}
}


fn handle_raw_shortcut_event( _key: u16, modifier: u8, previous: u8, current: u8, key_down: bool, event_time: u32 ) -> Option< GlobalInputAction > {
	if current_shortcut() == GlobalStartShortcut::Win { return None; }

	let win_key = modifier & WIN_DOWN != 0;

	if key_down && win_key && previous & WIN_DOWN == 0 {
		RAW_WIN_SEQUENCE_USED.store( false, Ordering::SeqCst );
		RAW_WIN_SEQUENCE_HANDLED.store( false, Ordering::SeqCst );
	}

	let shortcut = current_shortcut();

	if key_down && !win_key && previous & WIN_DOWN != 0 && !( shortcut == GlobalStartShortcut::WinShift && modifier & SHIFT_DOWN != 0 ) {
		RAW_WIN_SEQUENCE_USED.store( true, Ordering::SeqCst );
	}

	if shortcut == GlobalStartShortcut::WinShift && !chord_complete( previous ) && chord_complete( current ) {
		RAW_WIN_SEQUENCE_HANDLED.store( true, Ordering::SeqCst );
		if claim_toggle_event( event_time ) { return Some( GlobalInputAction::Toggle ); }
	}

	if !key_down && win_key && current & WIN_DOWN == 0 {
		let used = RAW_WIN_SEQUENCE_USED.swap( false, Ordering::SeqCst );
		let handled = RAW_WIN_SEQUENCE_HANDLED.swap( false, Ordering::SeqCst );

		return match shortcut {
			GlobalStartShortcut::WinShift if !handled && !used && SURFACE_VISIBLE.load( Ordering::SeqCst ) => Some( GlobalInputAction::Dismiss ),
			GlobalStartShortcut::Win if !used && !handled && claim_toggle_event( event_time ) => Some( GlobalInputAction::Toggle ),
			_ => None,
		};
	}

	None
}


fn current_shortcut() -> GlobalStartShortcut {
	if SHORTCUT_MODE.load( Ordering::SeqCst ) == GlobalStartShortcut::Win as u8 { GlobalStartShortcut::Win } else { GlobalStartShortcut::WinShift }
}


fn reset_input_state() {
	MODIFIER_STATE.store( 0, Ordering::SeqCst );
	RAW_MODIFIER_STATE.store( 0, Ordering::SeqCst );
	LAST_TOGGLE_EVENT_TIME.store( 0, Ordering::SeqCst );
	LAST_NATIVE_START_EVENT_TIME.store( 0, Ordering::SeqCst );
	ESCAPE_CAPTURED.store( false, Ordering::SeqCst );
	ALT_TAB_SEQUENCE_ACTIVE.store( false, Ordering::SeqCst );
	ALT_TAB_ESCAPE_CANCELLED.store( false, Ordering::SeqCst );
	reset_shortcut_state();
}


fn reset_shortcut_state() {
	WIN_SEQUENCE_USED.store( false, Ordering::SeqCst );
	WIN_SEQUENCE_HANDLED.store( false, Ordering::SeqCst );
	RAW_WIN_SEQUENCE_USED.store( false, Ordering::SeqCst );
	RAW_WIN_SEQUENCE_HANDLED.store( false, Ordering::SeqCst );
}


fn dismiss_native_start_menu() {
	let inputs = [
		keyboard_input( VK_ESCAPE, Default::default() ),
		keyboard_input( VK_ESCAPE, KEYEVENTF_KEYUP )
	];

	unsafe { let _ = SendInput( &inputs, size_of::< INPUT >() as i32 ); }
}


fn keyboard_input( key: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS ) -> INPUT {
	INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: key, dwFlags: flags, ..Default::default() } } }
}


fn is_native_start_menu_window( hwnd: HWND ) -> bool {
	let Some( image_path ) = window_process_image_path( hwnd ) else { return false; };
	image_path.ends_with( "\\startmenuexperiencehost.exe" ) || image_path == "startmenuexperiencehost.exe"
}


fn window_process_image_path( hwnd: HWND ) -> Option< String > {
	if hwnd.is_invalid() { return None; }
	let mut process_id = 0;
	unsafe { GetWindowThreadProcessId( hwnd, Some( &mut process_id ) ); }
	if process_id == 0 { return None; }
	let process = unsafe { OpenProcess( PROCESS_QUERY_LIMITED_INFORMATION, false, process_id ) }.ok()?;
	let mut buffer = [ 0u16; 32768 ];
	let mut length = buffer.len() as u32;
	let result = unsafe { QueryFullProcessImageNameW( process, PROCESS_NAME_FORMAT( 0 ), PWSTR( buffer.as_mut_ptr() ), &mut length ) };
	unsafe { let _ = CloseHandle( process ); }
	if result.is_err() || length == 0 { return None; }
	Some( String::from_utf16_lossy( &buffer[ ..length as usize ] ).to_ascii_lowercase() )
}


fn claim_native_start_event( event_time: u32 ) -> bool {
	let previous = LAST_NATIVE_START_EVENT_TIME.swap( event_time, Ordering::SeqCst );
	previous == 0 || event_time.wrapping_sub( previous ) > NATIVE_START_EVENT_GAP_MS
}


fn register_raw_keyboard( hwnd: HWND ) -> Result< (), String > {
	let device = RAWINPUTDEVICE { usUsagePage: 0x01, usUsage: 0x06, dwFlags: RIDEV_INPUTSINK, hwndTarget: hwnd };
	unsafe { RegisterRawInputDevices( &[ device ], size_of::< RAWINPUTDEVICE >() as u32 ) }.map_err( |error| format!( "注册后台键盘输入失败：{}", error ) )
}


fn unregister_raw_keyboard() {
	let device = RAWINPUTDEVICE { usUsagePage: 0x01, usUsage: 0x06, dwFlags: RIDEV_REMOVE, hwndTarget: HWND::default() };
	unsafe { let _ = RegisterRawInputDevices( &[ device ], size_of::< RAWINPUTDEVICE >() as u32 ); }
}


fn read_raw_keyboard( lparam: LPARAM ) -> Option< RAWKEYBOARD > {
	let mut input = RAWINPUT::default();
	let mut size = size_of::< RAWINPUT >() as u32;
	let result = unsafe { GetRawInputData( HRAWINPUT( lparam.0 as *mut c_void ), RID_INPUT, Some( ( &mut input as *mut RAWINPUT ).cast() ), &mut size, size_of::< windows::Win32::UI::Input::RAWINPUTHEADER >() as u32 ) };

	if result == u32::MAX || input.header.dwType != RIM_TYPEKEYBOARD.0 { return None; }

	Some( unsafe { input.data.keyboard } )
}


fn raw_modifier_mask( keyboard: &RAWKEYBOARD ) -> u8 {
	match keyboard.VKey {
		key if key == VK_SHIFT.0 => if keyboard.MakeCode == 0x36 { RIGHT_SHIFT_DOWN } else { LEFT_SHIFT_DOWN },
		key if key == VK_LSHIFT.0 => LEFT_SHIFT_DOWN,
		key if key == VK_RSHIFT.0 => RIGHT_SHIFT_DOWN,
		key if key == VK_LWIN.0 => LEFT_WIN_DOWN,
		key if key == VK_RWIN.0 => RIGHT_WIN_DOWN,
		key if key == VK_MENU.0 || key == VK_LMENU.0 => LEFT_ALT_DOWN,
		key if key == VK_RMENU.0 => RIGHT_ALT_DOWN,
		_ => 0,
	}
}


fn chord_complete( state: u8 ) -> bool {
	state & SHIFT_DOWN != 0 && state & WIN_DOWN != 0
}


fn modifier_mask( key: u32, scan_code: u32 ) -> u8 {
	match key as u16 {
		key if key == VK_SHIFT.0 => if scan_code == 0x36 { RIGHT_SHIFT_DOWN } else { LEFT_SHIFT_DOWN },
		key if key == VK_LSHIFT.0 => LEFT_SHIFT_DOWN,
		key if key == VK_RSHIFT.0 => RIGHT_SHIFT_DOWN,
		key if key == VK_LWIN.0 => LEFT_WIN_DOWN,
		key if key == VK_RWIN.0 => RIGHT_WIN_DOWN,
		key if key == VK_MENU.0 || key == VK_LMENU.0 => LEFT_ALT_DOWN,
		key if key == VK_RMENU.0 => RIGHT_ALT_DOWN,
		_ => 0,
	}
}


fn post_message( message: u32 ) {
	let hwnd = HWND( TARGET_WINDOW.load( Ordering::SeqCst ) );
	if hwnd.is_invalid() || message == 0 { return; }
	unsafe { let _ = PostMessageW( Some( hwnd ), message, WPARAM( 0 ), LPARAM( 0 ) ); }
}


fn post_alt_tab_event( event: GlobalAltTabEvent ) {
	let hwnd = HWND( TARGET_WINDOW.load( Ordering::SeqCst ) );
	let message = ALT_TAB_MESSAGE.load( Ordering::SeqCst );
	if hwnd.is_invalid() || message == 0 { return; }
	unsafe { let _ = PostMessageW( Some( hwnd ), message, WPARAM( event as usize ), LPARAM( 0 ) ); }
}


fn claim_toggle_event( event_time: u32 ) -> bool {
	let previous = LAST_TOGGLE_EVENT_TIME.swap( event_time, Ordering::SeqCst );
	previous == 0 || ( event_time.wrapping_sub( previous ) > 250 && previous.wrapping_sub( event_time ) > 250 )
}
