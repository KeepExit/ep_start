//! ::  Project Path  ->  ep_start :: foreground.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 16:56 周六


use std::ffi::c_void;
use std::sync::atomic::{ AtomicBool, AtomicPtr, AtomicU32, Ordering };
use windows::Win32::Foundation::{ HINSTANCE, HWND, LPARAM, LRESULT, WPARAM };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::{ HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent };
use windows::Win32::UI::WindowsAndMessaging::{ CallNextHookEx, EVENT_SYSTEM_FOREGROUND, HC_ACTION, HHOOK, MSLLHOOKSTRUCT, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx, WH_MOUSE_LL, WINEVENT_OUTOFCONTEXT, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_RBUTTONDOWN, WM_XBUTTONDOWN };


static TARGET_WINDOW: AtomicPtr< c_void > = AtomicPtr::new( std::ptr::null_mut() );
static CHANGE_MESSAGE: AtomicU32 = AtomicU32::new( 0 );
static ENABLED: AtomicBool = AtomicBool::new( false );
static LAST_ACTIVATION_INPUT_TIME: AtomicU32 = AtomicU32::new( 0 );


pub struct ForegroundChangeObserver {
	hook: HWINEVENTHOOK,
	mouse_hook: Option< HHOOK >,
	target: HWND,
}


impl ForegroundChangeObserver {
	pub fn watch( target: HWND, change_message: u32 ) -> Result< Self, String > {
		if target.is_invalid() || change_message == 0 { return Err( "无法使用无效窗口或消息监听前台切换".to_string() ); }
		TARGET_WINDOW.compare_exchange( std::ptr::null_mut(), target.0, Ordering::SeqCst, Ordering::SeqCst ).map_err( |_| "前台窗口监听已经被占用".to_string() )?;
		CHANGE_MESSAGE.store( change_message, Ordering::SeqCst );
		let hook = unsafe { SetWinEventHook( EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some( foreground_event ), 0, 0, WINEVENT_OUTOFCONTEXT ) };
		if hook.is_invalid() {
			CHANGE_MESSAGE.store( 0, Ordering::SeqCst );
			TARGET_WINDOW.store( std::ptr::null_mut(), Ordering::SeqCst );
			return Err( "注册前台窗口监听失败".to_string() );
		}
		Ok( Self { hook, mouse_hook: None, target } )
	}


	pub fn set_enabled( &mut self, enabled: bool ) {
		if enabled {
			if self.mouse_hook.is_some() { return; }
			LAST_ACTIVATION_INPUT_TIME.store( 0, Ordering::SeqCst );
			let hook = unsafe { GetModuleHandleW( None ) }.ok().and_then( |module| unsafe { SetWindowsHookExW( WH_MOUSE_LL, Some( mouse_event ), Some( HINSTANCE( module.0 ) ), 0 ) }.ok() );
			self.mouse_hook = hook;
			ENABLED.store( true, Ordering::SeqCst );
		} else {
			ENABLED.store( false, Ordering::SeqCst );
			if let Some( hook ) = self.mouse_hook.take() { unsafe { let _ = UnhookWindowsHookEx( hook ); } }
			LAST_ACTIVATION_INPUT_TIME.store( 0, Ordering::SeqCst );
		}
	}
}


impl Drop for ForegroundChangeObserver {
	fn drop( &mut self ) {
		self.set_enabled( false );
		unsafe { let _ = UnhookWinEvent( self.hook ); }
		CHANGE_MESSAGE.store( 0, Ordering::SeqCst );
		let _ = TARGET_WINDOW.compare_exchange( self.target.0, std::ptr::null_mut(), Ordering::SeqCst, Ordering::SeqCst );
	}
}


unsafe extern "system" fn foreground_event( _hook: HWINEVENTHOOK, event: u32, _foreground: HWND, _object_id: i32, _child_id: i32, _event_thread: u32, event_time: u32 ) {
	if event != EVENT_SYSTEM_FOREGROUND || !ENABLED.load( Ordering::SeqCst ) { return; }
	let input_time = LAST_ACTIVATION_INPUT_TIME.load( Ordering::SeqCst );
	if input_time == 0 || event_time.wrapping_sub( input_time ) > 1000 { return; }
	let target = HWND( TARGET_WINDOW.load( Ordering::SeqCst ) );
	let message = CHANGE_MESSAGE.load( Ordering::SeqCst );
	if target.is_invalid() || message == 0 { return; }
	unsafe { let _ = PostMessageW( Some( target ), message, WPARAM( 0 ), LPARAM( 0 ) ); }
}


unsafe extern "system" fn mouse_event( code: i32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if code == HC_ACTION as i32 && matches!( wparam.0 as u32, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN ) {
		let event = unsafe { &*( lparam.0 as *const MSLLHOOKSTRUCT ) };
		note_activation_input( event.time );
	}
	unsafe { CallNextHookEx( None, code, wparam, lparam ) }
}


pub(crate) fn note_activation_input( time: u32 ) {
	if ENABLED.load( Ordering::SeqCst ) { LAST_ACTIVATION_INPUT_TIME.store( time, Ordering::SeqCst ); }
}
