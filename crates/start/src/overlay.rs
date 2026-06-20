//! ::  Project Path  ->  ep_start :: overlay.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 20:46 周六


use platform::MonitorGeometry;
use windows::Win32::Foundation::{ COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM };
use windows::Win32::Graphics::Gdi::{ BLACK_BRUSH, GetStockObject, HBRUSH };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{ CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_TOPMOST, LWA_ALPHA, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetLayeredWindowAttributes, SetWindowPos, ShowWindow, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP };
use windows::core::{ Result as WindowsResult, w };


pub struct OverlaySurface {
	hwnd: HWND,
}


impl OverlaySurface {
	pub fn create() -> Result< Self, String > {
		let hwnd = unsafe { create_window() }.map_err( |error| format!( "创建 Start 遮罩窗口失败：{}", error ) )?;
		Ok( Self { hwnd } )
	}


	pub fn show( &self, geometry: &MonitorGeometry, insert_after: Option< HWND > ) {
		let order = insert_after.unwrap_or( HWND_TOPMOST );
		unsafe {
			let _ = SetWindowPos( self.hwnd, Some( order ), geometry.work_rect.left, geometry.work_rect.top, geometry.work_width(), geometry.work_height(), SWP_NOACTIVATE | SWP_SHOWWINDOW );
			let _ = ShowWindow( self.hwnd, SW_SHOWNOACTIVATE );
		}
	}


	pub fn set_opacity( &self, opacity: u8 ) {
		unsafe { let _ = SetLayeredWindowAttributes( self.hwnd, COLORREF( 0 ), opacity, LWA_ALPHA ); }
	}


	pub fn hide( &self ) {
		unsafe { let _ = ShowWindow( self.hwnd, SW_HIDE ); }
	}
}


impl Drop for OverlaySurface {
	fn drop( &mut self ) {
		unsafe { let _ = DestroyWindow( self.hwnd ); }
	}
}


unsafe fn create_window() -> WindowsResult< HWND > {
	let module = unsafe { GetModuleHandleW( None )? };
	let instance = HINSTANCE( module.0 );
	let class = WNDCLASSW { lpfnWndProc: Some( overlay_window_proc ), hInstance: instance, hbrBackground: HBRUSH( unsafe { GetStockObject( BLACK_BRUSH ) }.0 ), lpszClassName: w!( "EpStartOverlayWindow" ), ..Default::default() };
	if unsafe { RegisterClassW( &class ) } == 0 { return Err( windows::core::Error::from_thread() ); }
	let hwnd = unsafe { CreateWindowExW( WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW, w!( "EpStartOverlayWindow" ), w!( "ep_start overlay" ), WS_POPUP, 0, 0, 0, 0, None, None, Some( instance ), None )? };
	unsafe { SetLayeredWindowAttributes( hwnd, COLORREF( 0 ), 0, LWA_ALPHA )?; }
	Ok( hwnd )
}


unsafe extern "system" fn overlay_window_proc( hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	unsafe { DefWindowProcW( hwnd, message, wparam, lparam ) }
}
