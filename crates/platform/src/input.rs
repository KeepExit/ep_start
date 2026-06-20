//! ::  Project Path  ->  ep_start :: input.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 03:17 周六


use std::ffi::c_void;
use std::mem::size_of;
use std::sync::mpsc::{ SyncSender, sync_channel };
use std::sync::atomic::{ AtomicBool, AtomicPtr, AtomicU8, AtomicU32, Ordering };
use std::thread::{ self, JoinHandle };
use windows::Win32::Foundation::{ HINSTANCE, HWND, LPARAM, LRESULT, WPARAM };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::{ GetRawInputData, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWKEYBOARD, RIDEV_INPUTSINK, RIDEV_REMOVE, RID_INPUT, RIM_TYPEKEYBOARD, RegisterRawInputDevices };
use windows::Win32::UI::Input::KeyboardAndMouse::{ GetAsyncKeyState, VK_CONTROL, VK_ESCAPE, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_TAB };
use windows::Win32::UI::WindowsAndMessaging::{ CallNextHookEx, DispatchMessageW, GetMessageTime, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE, PeekMessageW, PostMessageW, PostThreadMessageW, RI_KEY_BREAK, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP };


const LEFT_SHIFT_DOWN: u8 = 1 << 0;
const RIGHT_SHIFT_DOWN: u8 = 1 << 1;
const LEFT_WIN_DOWN: u8 = 1 << 2;
const RIGHT_WIN_DOWN: u8 = 1 << 3;
const SHIFT_DOWN: u8 = LEFT_SHIFT_DOWN | RIGHT_SHIFT_DOWN;
const WIN_DOWN: u8 = LEFT_WIN_DOWN | RIGHT_WIN_DOWN;

static TARGET_WINDOW: AtomicPtr< c_void > = AtomicPtr::new( std::ptr::null_mut() );
static TOGGLE_MESSAGE: AtomicU32 = AtomicU32::new( 0 );
static DISMISS_MESSAGE: AtomicU32 = AtomicU32::new( 0 );
static SURFACE_VISIBLE: AtomicBool = AtomicBool::new( false );
static MODIFIER_STATE: AtomicU8 = AtomicU8::new( 0 );
static RAW_MODIFIER_STATE: AtomicU8 = AtomicU8::new( 0 );
static LAST_TOGGLE_EVENT_TIME: AtomicU32 = AtomicU32::new( 0 );
static ESCAPE_CAPTURED: AtomicBool = AtomicBool::new( false );


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


impl GlobalInputManager {
	pub fn new() -> Result< Self, String > {
		let ( ready_sender, ready_receiver ) = sync_channel( 1 );
		let hook_thread = thread::Builder::new().name( "ep-start-input-hook".to_string() ).spawn( move || run_hook_thread( ready_sender ) ).map_err( |error| format!( "创建全局输入线程失败：{}", error ) )?;
		match ready_receiver.recv().map_err( |error| format!( "等待全局输入线程失败：{}", error ) )? {
			Ok( hook_thread_id ) => Ok( Self { hook_thread_id, hook_thread: Some( hook_thread ) } ),
			Err( error ) => { let _ = hook_thread.join(); Err( error ) }
		}
	}


	pub fn bind_start_surface( &self, hwnd: HWND, toggle_message: u32, dismiss_message: u32 ) -> Result< GlobalInputBinding, String > {
		if hwnd.is_invalid() { return Err( "无法为无效窗口绑定 Start 快捷键".to_string() ); }
		reset_input_state();
		TOGGLE_MESSAGE.store( toggle_message, Ordering::SeqCst );
		DISMISS_MESSAGE.store( dismiss_message, Ordering::SeqCst );
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
	let hook = match unsafe { SetWindowsHookExW( WH_KEYBOARD_LL, Some( keyboard_hook ), Some( HINSTANCE( module.0 ) ), 0 ) } {
		Ok( hook ) => hook,
		Err( error ) => { let _ = ready.send( Err( format!( "安装全局输入监听失败：{}", error ) ) ); return; }
	};
	let mut message = MSG::default();
	unsafe { let _ = PeekMessageW( &mut message, None, 0, 0, PM_NOREMOVE ); }
	if ready.send( Ok( unsafe { GetCurrentThreadId() } ) ).is_err() { unsafe { let _ = UnhookWindowsHookEx( hook ); } return; }
	loop {
		let result = unsafe { GetMessageW( &mut message, None, 0, 0 ) };
		if result.0 <= 0 { break; }
		unsafe { let _ = TranslateMessage( &message ); DispatchMessageW( &message ); }
	}
	unsafe { let _ = UnhookWindowsHookEx( hook ); }
}


impl GlobalInputBinding {
	pub fn set_surface_visible( &self, visible: bool ) {
		SURFACE_VISIBLE.store( visible, Ordering::SeqCst );
	}


	pub fn raw_input_action( &self, lparam: LPARAM ) -> Option< GlobalInputAction > {
		let keyboard = read_raw_keyboard( lparam )?;
		let key_down = keyboard.Flags as u32 & RI_KEY_BREAK == 0;
		if keyboard.VKey == VK_ESCAPE.0 && key_down && SURFACE_VISIBLE.load( Ordering::SeqCst ) { return Some( GlobalInputAction::Dismiss ); }
		let modifier = raw_modifier_mask( &keyboard );
		if modifier == 0 { return None; }
		let previous = RAW_MODIFIER_STATE.load( Ordering::SeqCst );
		let current = if key_down { previous | modifier } else { previous & !modifier };
		RAW_MODIFIER_STATE.store( current, Ordering::SeqCst );
		if !chord_complete( previous ) && chord_complete( current ) {
			RAW_MODIFIER_STATE.store( 0, Ordering::SeqCst );
			if claim_toggle_event( unsafe { GetMessageTime() } as u32 ) { return Some( GlobalInputAction::Toggle ); }
		}
		None
	}
}


impl Drop for GlobalInputBinding {
	fn drop( &mut self ) {
		unregister_raw_keyboard();
		let _ = TARGET_WINDOW.compare_exchange( self.hwnd.0, std::ptr::null_mut(), Ordering::SeqCst, Ordering::SeqCst );
		TOGGLE_MESSAGE.store( 0, Ordering::SeqCst );
		DISMISS_MESSAGE.store( 0, Ordering::SeqCst );
		reset_input_state();
	}
}


unsafe extern "system" fn keyboard_hook( code: i32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if code == HC_ACTION as i32 {
		let event = unsafe { &*( lparam.0 as *const KBDLLHOOKSTRUCT ) };
		let message = wparam.0 as u32;
		let key_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
		let key_up = message == WM_KEYUP || message == WM_SYSKEYUP;
		if key_down && is_foreground_switch_key( event.vkCode as u16 ) { crate::foreground::note_activation_input( event.time ); }
		if event.vkCode as u16 == VK_ESCAPE.0 && handle_escape( key_down, key_up ) { return LRESULT( 1 ); }
		let modifier = modifier_mask( event.vkCode, event.scanCode );
		if modifier != 0 && ( key_down || key_up ) {
			if handle_modifier_event( modifier, key_down ) && claim_toggle_event( event.time ) { post_message( TOGGLE_MESSAGE.load( Ordering::SeqCst ) ); }
		}
	}
	unsafe { CallNextHookEx( None, code, wparam, lparam ) }
}


fn is_foreground_switch_key( key: u16 ) -> bool {
	key == VK_LWIN.0 || key == VK_RWIN.0 || ( key == VK_TAB.0 && unsafe { GetAsyncKeyState( VK_MENU.0 as i32 ) } < 0 ) || ( key == VK_ESCAPE.0 && unsafe { GetAsyncKeyState( VK_CONTROL.0 as i32 ) } < 0 )
}


fn handle_escape( key_down: bool, key_up: bool ) -> bool {
	if key_down && SURFACE_VISIBLE.load( Ordering::SeqCst ) {
		if !ESCAPE_CAPTURED.swap( true, Ordering::SeqCst ) { post_message( DISMISS_MESSAGE.load( Ordering::SeqCst ) ); }
		return true;
	}
	if key_up && ESCAPE_CAPTURED.swap( false, Ordering::SeqCst ) { return true; }
	false
}


fn handle_modifier_event( modifier: u8, key_down: bool ) -> bool {
	let previous = MODIFIER_STATE.load( Ordering::SeqCst );
	let was_complete = chord_complete( previous );
	let current = if key_down { previous | modifier } else { previous & !modifier };
	let is_complete = chord_complete( current );
	MODIFIER_STATE.store( current, Ordering::SeqCst );
	if !was_complete && is_complete {
		MODIFIER_STATE.store( 0, Ordering::SeqCst );
		return true;
	}
	false
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
		_ => 0,
	}
}


fn post_message( message: u32 ) {
	let hwnd = HWND( TARGET_WINDOW.load( Ordering::SeqCst ) );
	if hwnd.is_invalid() || message == 0 { return; }
	unsafe { let _ = PostMessageW( Some( hwnd ), message, WPARAM( 0 ), LPARAM( 0 ) ); }
}


fn claim_toggle_event( event_time: u32 ) -> bool {
	let previous = LAST_TOGGLE_EVENT_TIME.swap( event_time, Ordering::SeqCst );
	previous == 0 || ( event_time.wrapping_sub( previous ) > 50 && previous.wrapping_sub( event_time ) > 50 )
}


fn reset_input_state() {
	SURFACE_VISIBLE.store( false, Ordering::SeqCst );
	MODIFIER_STATE.store( 0, Ordering::SeqCst );
	RAW_MODIFIER_STATE.store( 0, Ordering::SeqCst );
	LAST_TOGGLE_EVENT_TIME.store( 0, Ordering::SeqCst );
	ESCAPE_CAPTURED.store( false, Ordering::SeqCst );
}
