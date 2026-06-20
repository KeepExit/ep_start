//! ::  Project Path  ->  ep_start :: transition.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 17:57 周六


use platform::MonitorGeometry;
use std::ffi::c_void;
use windows::Win32::Foundation::{ COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM };
use windows::Win32::Graphics::Gdi::{ BLACK_BRUSH, BeginPaint, BitBlt, CAPTUREBLT, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, EndPaint, GetDC, GetStockObject, HBITMAP, HBRUSH, HDC, HGDIOBJ, PAINTSTRUCT, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow, ReleaseDC, SRCCOPY, SelectObject };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{ CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, GWLP_USERDATA, GetWindowLongPtrW, HWND_TOPMOST, LWA_ALPHA, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_ERASEBKGND, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP };
use windows::core::{ Result as WindowsResult, w };


pub struct DesktopTransition {
	hwnd: HWND,
	state: *mut SnapshotState,
}


struct SnapshotState {
	resources: Option< SnapshotResources >,
}


struct SnapshotResources {
	dc: HDC,
	bitmap: HBITMAP,
	previous_bitmap: HGDIOBJ,
	width: i32,
	height: i32,
}


impl DesktopTransition {
	pub fn create() -> Result< Self, String > {
		let state = Box::into_raw( Box::new( SnapshotState { resources: None } ) );
		let hwnd = match unsafe { create_window( state ) } {
			Ok( hwnd ) => hwnd,
			Err( error ) => {
				unsafe { drop( Box::from_raw( state ) ); }
				return Err( format!( "创建桌面过渡窗口失败：{}", error ) );
			}
		};
		Ok( Self { hwnd, state } )
	}


	pub fn capture( &mut self, geometry: &MonitorGeometry ) -> Result< (), String > {
		self.release_snapshot();
		let resources = SnapshotResources::capture( geometry )?;
		unsafe { ( *self.state ).resources = Some( resources ); }
		Ok( () )
	}


	pub fn show( &self, geometry: &MonitorGeometry ) {
		if !self.is_ready() { return; }
		unsafe {
			let _ = SetLayeredWindowAttributes( self.hwnd, COLORREF( 0 ), 255, LWA_ALPHA );
			let _ = SetWindowPos( self.hwnd, Some( HWND_TOPMOST ), geometry.work_rect.left, geometry.work_rect.top, geometry.work_width(), geometry.work_height(), SWP_NOACTIVATE | SWP_SHOWWINDOW );
			let _ = ShowWindow( self.hwnd, SW_SHOWNOACTIVATE );
			let _ = RedrawWindow( Some( self.hwnd ), None, None, RDW_ERASE | RDW_INVALIDATE | RDW_UPDATENOW );
		}
	}


	pub fn set_opacity( &self, opacity: u8 ) {
		if self.is_ready() { unsafe { let _ = SetLayeredWindowAttributes( self.hwnd, COLORREF( 0 ), opacity, LWA_ALPHA ); } }
	}


	pub fn cover_window( &self ) -> Option< HWND > {
		self.is_ready().then_some( self.hwnd )
	}


	pub fn discard( &mut self ) {
		unsafe { let _ = ShowWindow( self.hwnd, SW_HIDE ); }
		self.release_snapshot();
	}


	pub fn hide( &mut self ) {
		self.discard();
	}


	fn is_ready( &self ) -> bool {
		unsafe { ( *self.state ).resources.is_some() }
	}


	fn release_snapshot( &mut self ) {
		unsafe { drop( ( *self.state ).resources.take() ); }
	}
}


impl Drop for DesktopTransition {
	fn drop( &mut self ) {
		self.hide();
		unsafe {
			let _ = DestroyWindow( self.hwnd );
			drop( Box::from_raw( self.state ) );
		}
	}
}


impl SnapshotResources {
	fn capture( geometry: &MonitorGeometry ) -> Result< Self, String > {
		let width = geometry.work_width();
		let height = geometry.work_height();
		let screen = unsafe { GetDC( None ) };
		if screen.is_invalid() { return Err( "获取屏幕绘制上下文失败".to_string() ); }
		let dc = unsafe { CreateCompatibleDC( Some( screen ) ) };
		if dc.is_invalid() {
			unsafe { let _ = ReleaseDC( None, screen ); }
			return Err( "创建过渡绘制上下文失败".to_string() );
		}
		let bitmap = unsafe { CreateCompatibleBitmap( screen, width, height ) };
		if bitmap.is_invalid() {
			unsafe { let _ = DeleteDC( dc ); let _ = ReleaseDC( None, screen ); }
			return Err( "创建桌面过渡位图失败".to_string() );
		}
		let previous_bitmap = unsafe { SelectObject( dc, HGDIOBJ( bitmap.0 ) ) };
		let result = unsafe { BitBlt( dc, 0, 0, width, height, Some( screen ), geometry.work_rect.left, geometry.work_rect.top, SRCCOPY | CAPTUREBLT ) };
		unsafe { let _ = ReleaseDC( None, screen ); }
		if let Err( error ) = result {
			unsafe { let _ = SelectObject( dc, previous_bitmap ); let _ = DeleteObject( HGDIOBJ( bitmap.0 ) ); let _ = DeleteDC( dc ); }
			return Err( format!( "捕获桌面过渡画面失败：{}", error ) );
		}
		Ok( Self { dc, bitmap, previous_bitmap, width, height } )
	}
}


impl Drop for SnapshotResources {
	fn drop( &mut self ) {
		unsafe {
			let _ = SelectObject( self.dc, self.previous_bitmap );
			let _ = DeleteObject( HGDIOBJ( self.bitmap.0 ) );
			let _ = DeleteDC( self.dc );
		}
	}
}


unsafe fn create_window( state: *mut SnapshotState ) -> WindowsResult< HWND > {
	let module = unsafe { GetModuleHandleW( None )? };
	let instance = HINSTANCE( module.0 );
	let class = WNDCLASSW { lpfnWndProc: Some( transition_window_proc ), hInstance: instance, hbrBackground: HBRUSH( unsafe { GetStockObject( BLACK_BRUSH ) }.0 ), lpszClassName: w!( "EpStartTransitionWindow" ), ..Default::default() };
	if unsafe { RegisterClassW( &class ) } == 0 { return Err( windows::core::Error::from_thread() ); }
	unsafe { CreateWindowExW( WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, w!( "EpStartTransitionWindow" ), w!( "ep_start_transition" ), WS_POPUP, 0, 0, 0, 0, None, None, Some( instance ), Some( state.cast::< c_void >() ) ) }
}


unsafe extern "system" fn transition_window_proc( hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if message == WM_NCCREATE {
		let creation = unsafe { &*( lparam.0 as *const CREATESTRUCTW ) };
		unsafe { SetWindowLongPtrW( hwnd, GWLP_USERDATA, creation.lpCreateParams as isize ); }
	}
	let state = unsafe { GetWindowLongPtrW( hwnd, GWLP_USERDATA ) as *mut SnapshotState };
	match message {
		WM_PAINT => {
			let mut paint = PAINTSTRUCT::default();
			unsafe {
				BeginPaint( hwnd, &mut paint );
				if let Some( resources ) = ( !state.is_null() ).then( || ( *state ).resources.as_ref() ).flatten() { let _ = BitBlt( paint.hdc, 0, 0, resources.width, resources.height, Some( resources.dc ), 0, 0, SRCCOPY ); }
				let _ = EndPaint( hwnd, &paint );
			}
			return LRESULT( 0 );
		}
		WM_ERASEBKGND => { return LRESULT( 1 ); }
		WM_NCDESTROY => { unsafe { SetWindowLongPtrW( hwnd, GWLP_USERDATA, 0 ); } }
		_ => {}
	}
	unsafe { DefWindowProcW( hwnd, message, wparam, lparam ) }
}
