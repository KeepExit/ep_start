//! ::  Project Path  ->  ep_start :: menu_window.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 15:35 周六


use std::mem::size_of;
use std::sync::atomic::{ AtomicBool, Ordering };
use windows::Win32::Foundation::{ HWND, LPARAM, LRESULT, WPARAM };
use windows::Win32::Graphics::Dwm::{ DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUNDSMALL, DwmSetWindowAttribute };
use windows::Win32::Graphics::Gdi::{ CreateRoundRectRgn, DeleteObject, HGDIOBJ, SetWindowRgn };
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{ CWPSTRUCT, CallNextHookEx, GetClassNameW, GetWindowRect, HHOOK, SetWindowsHookExW, UnhookWindowsHookEx, WH_CALLWNDPROC, WM_SHOWWINDOW, WM_WINDOWPOSCHANGED };


static DARK_MODE: AtomicBool = AtomicBool::new( false );


pub struct MenuWindowStyler {
	hook: Option< HHOOK >,
}


impl MenuWindowStyler {
	pub fn install( dark: bool ) -> Self {
		DARK_MODE.store( dark, Ordering::SeqCst );
		let hook = unsafe { SetWindowsHookExW( WH_CALLWNDPROC, Some( menu_window_hook ), None, GetCurrentThreadId() ) }.ok();
		Self { hook }
	}
}


impl Drop for MenuWindowStyler {
	fn drop( &mut self ) {
		if let Some( hook ) = self.hook.take() { unsafe { let _ = UnhookWindowsHookEx( hook ); } }
	}
}


unsafe extern "system" fn menu_window_hook( code: i32, _wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if code >= 0 {
		let message = unsafe { &*( lparam.0 as *const CWPSTRUCT ) };
		let mut class_name = [ 0u16; 32 ];
		let length = unsafe { GetClassNameW( message.hwnd, &mut class_name ) };
		if ( message.message == WM_SHOWWINDOW || message.message == WM_WINDOWPOSCHANGED ) && length > 0 && String::from_utf16_lossy( &class_name[ ..length as usize ] ) == "#32768" {
			apply_style( message.hwnd );
			apply_rounded_region( message.hwnd );
		}
	}
	unsafe { CallNextHookEx( None, code, _wparam, lparam ) }
}


fn apply_rounded_region( hwnd: HWND ) {
	let mut rect = windows::Win32::Foundation::RECT::default();
	if unsafe { GetWindowRect( hwnd, &mut rect ) }.is_err() { return; }
	let dpi = unsafe { GetDpiForWindow( hwnd ) }.max( 96 ) as i32;
	let radius = 8 * dpi / 96;
	let region = unsafe { CreateRoundRectRgn( 0, 0, rect.right - rect.left + 1, rect.bottom - rect.top + 1, radius, radius ) };
	if unsafe { SetWindowRgn( hwnd, Some( region ), true ) } == 0 { unsafe { let _ = DeleteObject( HGDIOBJ( region.0 ) ); } }
}


fn apply_style( hwnd: HWND ) {
	let corner = DWMWCP_ROUNDSMALL;
	let dark = DARK_MODE.load( Ordering::SeqCst ) as i32;
	unsafe {
		let _ = DwmSetWindowAttribute( hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, std::ptr::from_ref( &corner ).cast(), size_of_val( &corner ) as u32 );
		let _ = DwmSetWindowAttribute( hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, std::ptr::from_ref( &dark ).cast(), size_of::< i32 >() as u32 );
	}
}
