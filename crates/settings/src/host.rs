//! ::  Project Path  ->  ep_start :: host :: window_host
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 13:38 周日


use crate::state::{PointerUpAction, SettingsState };
use crate::ui::geometry::scale;
use platform::trim_working_set;
use std::ffi::c_void;
use std::mem::size_of;
use windows::Win32::Foundation::{ HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM };
use windows::Win32::Graphics::Dwm::{ DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute };
use windows::Win32::Graphics::Gdi::{ BeginPaint, EndPaint, HMONITOR, InvalidateRect, PAINTSTRUCT };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{ GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI };
use windows::Win32::UI::Input::KeyboardAndMouse::{ ReleaseCapture, SetCapture };
use windows::Win32::UI::WindowsAndMessaging::{ CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, GetWindowRect, HICON, ICON_SMALL, IDC_ARROW, LoadCursorW, MINMAXINFO, PostMessageW, RegisterClassW, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SendMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_APP, WM_CAPTURECHANGED, WM_CLOSE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_EXITSIZEMOVE, WM_GETMINMAXINFO, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SETICON, WM_SETTINGCHANGE, WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW };
use windows::core::{ Result as WindowsResult, w };


const WM_SHOW_SETTINGS: u32 = WM_APP + 50;

pub( crate ) struct SettingsWindowHost {
	hwnd: HWND,
}

impl SettingsWindowHost {
	pub( crate ) unsafe fn create( state: *mut SettingsState, large_icon: HICON, small_icon: HICON ) -> WindowsResult< Self > {
		let module = unsafe { GetModuleHandleW( None )? };
		let instance = HINSTANCE( module.0 );
		let class = WNDCLASSW { lpfnWndProc: Some( settings_window_proc ), hInstance: instance, hIcon: large_icon, hCursor: unsafe { LoadCursorW( None, IDC_ARROW )? }, lpszClassName: w!( "EpStartSettingsWindow" ), ..Default::default() };
		if unsafe { RegisterClassW( &class ) } == 0 {
			return Err( windows::core::Error::from_thread() );
		}
		let hwnd = unsafe { CreateWindowExW( Default::default(), w!( "EpStartSettingsWindow" ), w!( "ep_start" ), WS_OVERLAPPEDWINDOW, 0, 0, 1100, 720, None, None, Some( instance ), Some( state.cast::< c_void >() ) )? };
		unsafe { SendMessageW( hwnd, WM_SETICON, Some( WPARAM( ICON_SMALL as usize ) ), Some( LPARAM( small_icon.0 as isize ) ) ); }
		Ok( Self { hwnd } )
	}
	pub( crate ) const fn hwnd( &self ) -> HWND {
		self.hwnd
	}
	pub( crate ) fn destroy( &self ) {
		unsafe { let _ = DestroyWindow( self.hwnd ); }
	}
}

pub( crate ) fn post_show_settings( hwnd: HWND ) {
	unsafe { let _ = PostMessageW( Some( hwnd ), WM_SHOW_SETTINGS, WPARAM( 0 ), LPARAM( 0 ) ); }
}
pub( crate ) fn show_window( hwnd: HWND ) {
	unsafe { let _ = ShowWindow( hwnd, SW_SHOW ); }
}
pub( crate ) fn hide_window( hwnd: HWND ) {
	unsafe { let _ = ShowWindow( hwnd, SW_HIDE ); }
}
pub( crate ) fn foreground_window( hwnd: HWND ) {
	unsafe { let _ = SetForegroundWindow( hwnd ); }
}
pub( crate ) fn set_window_bounds( hwnd: HWND, x: i32, y: i32, width: i32, height: i32 ) {
	unsafe { let _ = SetWindowPos( hwnd, None, x, y, width, height, Default::default() ); }
}
pub( crate ) fn set_window_bounds_no_activate( hwnd: HWND, rect: RECT ) {
	unsafe { let _ = SetWindowPos( hwnd, None, rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top, SWP_NOACTIVATE ); }
}
pub( crate ) fn request_repaint( hwnd: HWND ) {
	unsafe { let _ = InvalidateRect( Some( hwnd ), None, false ); }
}
pub( crate ) fn set_dark_frame( hwnd: HWND, dark: bool ) {
	let dark = dark as i32;
	unsafe { let _ = DwmSetWindowAttribute( hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, std::ptr::from_ref( &dark ).cast(), size_of::< i32 >() as u32 ); }
}
pub( crate ) fn dpi_for_window( hwnd: HWND ) -> i32 {
	unsafe { GetDpiForWindow( hwnd ) }.max( 96 ) as i32
}
pub( crate ) fn dpi_for_monitor( monitor: HMONITOR ) -> ( u32, u32 ) {
	let mut dpi_x = 96_u32;
	let mut dpi_y = 96_u32;
	unsafe { let _ = GetDpiForMonitor( monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y ); }
	( dpi_x, dpi_y )
}
pub( crate ) fn client_rect( hwnd: HWND ) -> RECT {
	let mut client = RECT::default();
	unsafe { let _ = GetClientRect( hwnd, &mut client ); }
	client
}
pub( crate ) fn window_rect( hwnd: HWND ) -> Option< RECT > {
	let mut rect = RECT::default();
	unsafe { GetWindowRect( hwnd, &mut rect ) }.ok().map( |_| rect )
}
pub( crate ) fn capture_mouse( hwnd: HWND ) {
	unsafe { SetCapture( hwnd ); }
}
pub( crate ) fn release_mouse() {
	unsafe { let _ = ReleaseCapture(); }
}
pub( crate ) fn paint_window( hwnd: HWND, state: &SettingsState ) {
	let mut paint = PAINTSTRUCT::default();
	let mut client = RECT::default();
	unsafe {
		BeginPaint( hwnd, &mut paint );
		let _ = GetClientRect( hwnd, &mut client );
		state.paint_buffered( paint.hdc, client );
		let _ = EndPaint( hwnd, &paint );
	}
}

unsafe extern "system" fn settings_window_proc( hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if message == WM_NCCREATE {
		let creation = unsafe { &*( lparam.0 as *const CREATESTRUCTW ) };
		let state = creation.lpCreateParams.cast::< SettingsState >();
		unsafe {
			( *state ).set_hwnd( hwnd );
			SetWindowLongPtrW( hwnd, GWLP_USERDATA, state as isize );
		}
	}
	let state = unsafe { GetWindowLongPtrW( hwnd, GWLP_USERDATA ) as *mut SettingsState };
	if !state.is_null() {
		match message {
			WM_SHOW_SETTINGS => {
				unsafe { ( *state ).show(); }
				return LRESULT( 0 );
			}
			WM_CLOSE => {
				hide_window( hwnd );
				trim_working_set();
				return LRESULT( 0 );
			}
			WM_LBUTTONDOWN => {
				let ( x, y ) = point_from_lparam( lparam );
				if unsafe { ( *state ).on_pointer_down( x, y ) } {
					capture_mouse( hwnd );
				}
				return LRESULT( 0 );
			}
			WM_MOUSEMOVE => {
				let ( x, y ) = point_from_lparam( lparam );
				unsafe { ( *state ).on_pointer_move( x, y ); }
				return LRESULT( 0 );
			}
			WM_LBUTTONUP => {
				let ( x, y ) = point_from_lparam( lparam );
				match unsafe { ( *state ).on_pointer_up( x, y ) } {
					PointerUpAction::None => {}
					PointerUpAction::ReleaseCapture => release_mouse(),
					PointerUpAction::Choice( field ) => unsafe { ( *state ).choose( field ); },
				}
				return LRESULT( 0 );
			}
			WM_CAPTURECHANGED => {
				unsafe { ( *state ).on_capture_changed(); }
				return LRESULT( 0 );
			}
			WM_PAINT => {
				unsafe { paint_window( hwnd, &*state ); }
				return LRESULT( 0 );
			}
			WM_SIZE => {
				unsafe { ( *state ).on_size(); }
				return LRESULT( 0 );
			}
			WM_EXITSIZEMOVE => {
				unsafe { ( *state ).save_window_size(); }
				return LRESULT( 0 );
			}
			WM_MOUSEWHEEL => {
				let delta = ( wparam.0 >> 16 ) as i16 as i32;
				unsafe { ( *state ).on_mouse_wheel( delta ); }
				return LRESULT( 0 );
			}
			WM_GETMINMAXINFO => {
				let dpi = dpi_for_window( hwnd );
				let info = unsafe { &mut *( lparam.0 as *mut MINMAXINFO ) };
				info.ptMinTrackSize.x = scale( 620, dpi );
				info.ptMinTrackSize.y = scale( 460, dpi );
				return LRESULT( 0 );
			}
			WM_SETTINGCHANGE => {
				unsafe { ( *state ).refresh_theme(); }
				return LRESULT( 0 );
			}
			WM_DPICHANGED => {
				let suggested = unsafe { *( lparam.0 as *const RECT ) };
				set_window_bounds_no_activate( hwnd, suggested );
				request_repaint( hwnd );
				return LRESULT( 0 );
			}
			WM_ERASEBKGND => { return LRESULT( 1 ); }
			WM_DESTROY => { return LRESULT( 0 ); }
			WM_NCDESTROY => { unsafe { SetWindowLongPtrW( hwnd, GWLP_USERDATA, 0 ); } }
			_ => {}
		}
	}
	unsafe { DefWindowProcW( hwnd, message, wparam, lparam ) }
}

fn point_from_lparam( lparam: LPARAM ) -> ( i32, i32 ) {
	( lparam.0 as i16 as i32, ( lparam.0 >> 16 ) as i16 as i32 )
}
