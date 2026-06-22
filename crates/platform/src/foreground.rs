//! ::  Project Path  ->  ep_start :: foreground.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 16:56 周六


use std::ffi::c_void;
use std::sync::atomic::{ AtomicBool, AtomicPtr, AtomicU32, Ordering };
use windows::Win32::Foundation::{ HWND, LPARAM, WPARAM };
use windows::Win32::UI::Accessibility::{ HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent };
use windows::Win32::UI::WindowsAndMessaging::{ EVENT_SYSTEM_FOREGROUND, PostMessageW, WINEVENT_OUTOFCONTEXT };


static TARGET_WINDOW: AtomicPtr< c_void > = AtomicPtr::new( std::ptr::null_mut() );
static CHANGE_MESSAGE: AtomicU32 = AtomicU32::new( 0 );
static ENABLED: AtomicBool = AtomicBool::new( false );


pub struct ForegroundChangeObserver {
	hook: HWINEVENTHOOK,
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
		Ok( Self { hook, target } )
	}


	pub fn set_enabled( &mut self, enabled: bool ) {
		ENABLED.store( enabled, Ordering::SeqCst );
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


unsafe extern "system" fn foreground_event( _hook: HWINEVENTHOOK, event: u32, foreground: HWND, _object_id: i32, _child_id: i32, _event_thread: u32, _event_time: u32 ) {
	if event != EVENT_SYSTEM_FOREGROUND || !ENABLED.load( Ordering::SeqCst ) { return; }
	let target = HWND( TARGET_WINDOW.load( Ordering::SeqCst ) );
	let message = CHANGE_MESSAGE.load( Ordering::SeqCst );
	if target.is_invalid() || foreground.is_invalid() || message == 0 { return; }
	unsafe { let _ = PostMessageW( Some( target ), message, WPARAM( foreground.0 as usize ), LPARAM( 0 ) ); }
}
