//! ::  Project Path  ->  ep_start :: transition.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 17:57 周六


use platform::MonitorGeometry;
use std::ffi::c_void;
use std::mem::size_of;
use windows::Win32::Foundation::{ COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM };
use windows::Win32::Graphics::Dwm::{ DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute };
use windows::Win32::Graphics::Gdi::{ BLACK_BRUSH, BeginPaint, BitBlt, CAPTUREBLT, CombineRgn, CreateCompatibleBitmap, CreateCompatibleDC, CreateRectRgn, DeleteDC, DeleteObject, EndPaint, GetDC, GetStockObject, HBITMAP, HBRUSH, HDC, HGDIOBJ, PAINTSTRUCT, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW, RGN_OR, RedrawWindow, ReleaseDC, SRCCOPY, SelectObject, SetWindowRgn };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{ CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, EnumWindows, GWL_EXSTYLE, GWLP_USERDATA, GetClassNameW, GetWindowLongPtrW, GetWindowLongW, GetWindowRect, HWND_TOPMOST, IsIconic, IsWindowVisible, LWA_ALPHA, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_ERASEBKGND, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP };
use windows::core::{ BOOL, Result as WindowsResult, w };


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
	clips: Vec< RECT >,
}


struct WindowEnumeration {
	work_rect: RECT,
	clips: Vec< RECT >,
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
			if let Some( resources ) = ( *self.state ).resources.as_ref() { apply_window_regions( self.hwnd, &resources.clips ); }
			let _ = SetWindowPos( self.hwnd, Some( HWND_TOPMOST ), geometry.work_rect.left, geometry.work_rect.top, geometry.work_width(), geometry.work_height(), SWP_NOACTIVATE );
			let _ = RedrawWindow( Some( self.hwnd ), None, None, RDW_ERASE | RDW_INVALIDATE | RDW_UPDATENOW );
			let _ = ShowWindow( self.hwnd, SW_SHOWNOACTIVATE );
		}
	}


	pub fn hwnd( &self ) -> HWND {
		self.hwnd
	}


	pub fn set_opacity( &self, opacity: u8 ) {
		if self.is_ready() { unsafe { let _ = SetLayeredWindowAttributes( self.hwnd, COLORREF( 0 ), opacity, LWA_ALPHA ); } }
	}


	pub fn discard( &mut self ) {
		unsafe { let _ = ShowWindow( self.hwnd, SW_HIDE ); let _ = SetWindowRgn( self.hwnd, None, false ); }
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
		let clips = visible_window_clips( geometry.work_rect );
		if clips.is_empty() { return Err( "当前工作区没有需要过渡的可见窗口".to_string() ); }
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
		Ok( Self { dc, bitmap, previous_bitmap, width, height, clips } )
	}
}


fn visible_window_clips( work_rect: RECT ) -> Vec< RECT > {
	let mut enumeration = WindowEnumeration { work_rect, clips: Vec::new() };
	unsafe { let _ = EnumWindows( Some( collect_visible_window ), LPARAM( ( &mut enumeration as *mut WindowEnumeration ) as isize ) ); }
	enumeration.clips
}


unsafe extern "system" fn collect_visible_window( hwnd: HWND, lparam: LPARAM ) -> BOOL {
	let enumeration = unsafe { &mut *( lparam.0 as *mut WindowEnumeration ) };
	if !unsafe { IsWindowVisible( hwnd ) }.as_bool() || unsafe { IsIconic( hwnd ) }.as_bool() { return true.into(); }
	let class_name = window_class_name( hwnd );
	if matches!( class_name.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "EpStartTransitionWindow" | "EpStartBackdropWindow" | "EpStartOverlayWindow" | "Windows.UI.EpStartWindow" ) { return true.into(); }
	let extended_style = unsafe { GetWindowLongW( hwnd, GWL_EXSTYLE ) } as u32;
	if extended_style & ( WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0 ) != 0 { return true.into(); }
	let mut cloaked = 0u32;
	if unsafe { DwmGetWindowAttribute( hwnd, DWMWA_CLOAKED, ( &mut cloaked as *mut u32 ).cast(), size_of::< u32 >() as u32 ) }.is_ok() && cloaked != 0 { return true.into(); }
	let mut bounds = RECT::default();
	if unsafe { DwmGetWindowAttribute( hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, ( &mut bounds as *mut RECT ).cast(), size_of::< RECT >() as u32 ) }.is_err() && unsafe { GetWindowRect( hwnd, &mut bounds ) }.is_err() { return true.into(); }
	if let Some( clip ) = relative_clip( bounds, enumeration.work_rect ) { enumeration.clips.push( clip ); }
	true.into()
}


fn relative_clip( bounds: RECT, work_rect: RECT ) -> Option< RECT > {
	let left = bounds.left.max( work_rect.left );
	let top = bounds.top.max( work_rect.top );
	let right = bounds.right.min( work_rect.right );
	let bottom = bounds.bottom.min( work_rect.bottom );
	if right <= left || bottom <= top { return None; }
	Some( RECT { left: left - work_rect.left, top: top - work_rect.top, right: right - work_rect.left, bottom: bottom - work_rect.top } )
}


fn window_class_name( hwnd: HWND ) -> String {
	let mut class_name = [ 0u16; 128 ];
	let length = unsafe { GetClassNameW( hwnd, &mut class_name ) };
	if length <= 0 { return String::new(); }
	String::from_utf16_lossy( &class_name[ ..length as usize ] )
}


unsafe fn apply_window_regions( hwnd: HWND, clips: &[ RECT ] ) {
	let combined = unsafe { CreateRectRgn( 0, 0, 0, 0 ) };
	if combined.is_invalid() { return; }
	for clip in clips {
		let region = unsafe { CreateRectRgn( clip.left, clip.top, clip.right, clip.bottom ) };
		if region.is_invalid() { continue; }
		unsafe { let _ = CombineRgn( Some( combined ), Some( combined ), Some( region ), RGN_OR ); let _ = DeleteObject( HGDIOBJ( region.0 ) ); }
	}
	if unsafe { SetWindowRgn( hwnd, Some( combined ), false ) } == 0 { unsafe { let _ = DeleteObject( HGDIOBJ( combined.0 ) ); } }
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


#[cfg( test )]
mod tests {
	use super::*;


	#[test]
	fn non_maximized_window_clip_preserves_its_visible_area() {
		let work = RECT { left: 100, top: 50, right: 1100, bottom: 750 };
		let bounds = RECT { left: 260, top: 140, right: 860, bottom: 640 };
		assert_eq!( relative_clip( bounds, work ), Some( RECT { left: 160, top: 90, right: 760, bottom: 590 } ) );
	}


	#[test]
	fn transition_clip_is_limited_to_the_work_area() {
		let work = RECT { left: 0, top: 40, right: 1920, bottom: 1080 };
		let bounds = RECT { left: -8, top: 32, right: 1928, bottom: 1088 };
		assert_eq!( relative_clip( bounds, work ), Some( RECT { left: 0, top: 0, right: 1920, bottom: 1040 } ) );
	}
}
