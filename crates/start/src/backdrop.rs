//! ::  Project Path  ->  ep_start :: backdrop.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 03:17 周六


use platform::MonitorGeometry;
use crate::backdrop_capture::DesktopCapture;
use windows::Win32::Foundation::{ HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM };
use windows::Win32::Graphics::Dwm::{ DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION, DWM_TNP_RECTSOURCE, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE, DwmFlush, DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties };
use windows::Win32::Graphics::Gdi::{ BLACK_BRUSH, GetStockObject, HBRUSH };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{ CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW, GetWindowRect, HWND_TOPMOST, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetWindowPos, ShowWindow, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP };
use windows::core::{ PCWSTR, Result as WindowsResult, w };


pub struct DesktopBackdrop {
	hwnd: HWND,
	source: Option< HWND >,
	thumbnail: Option< isize >,
	capture: Option< DesktopCapture >,
	blur_percent: u8,
}


impl DesktopBackdrop {
	pub fn create() -> Result< Self, String > {
		let hwnd = unsafe { create_window() }.map_err( |error| format!( "创建桌面背景窗口失败：{}", error ) )?;
		Ok( Self { hwnd, source: None, thumbnail: None, capture: None, blur_percent: 0 } )
	}


	pub fn show( &mut self, geometry: &MonitorGeometry, cover: Option< HWND >, blur_percent: u8 ) {
		self.blur_percent = blur_percent.min( 100 );
		self.prepare_source( geometry );
		unsafe {
			let insert_after = cover.unwrap_or( HWND_TOPMOST );
			let _ = SetWindowPos( self.hwnd, Some( insert_after ), geometry.work_rect.left, geometry.work_rect.top, geometry.work_width(), geometry.work_height(), SWP_NOACTIVATE | SWP_SHOWWINDOW );
			let _ = ShowWindow( self.hwnd, SW_SHOWNOACTIVATE );
			let _ = DwmFlush();
		}
	}


	pub fn set_blur( &mut self, geometry: &MonitorGeometry, blur_percent: u8 ) {
		self.blur_percent = blur_percent.min( 100 );
		self.prepare_source( geometry );
	}


	pub fn update_frame( &mut self, _geometry: &MonitorGeometry ) -> bool {
		let Some( capture ) = &mut self.capture else { return false; };
		capture.present_next_frame()
	}


	pub fn hide( &mut self ) {
		unsafe { let _ = ShowWindow( self.hwnd, SW_HIDE ); }
		self.release_thumbnail();
		self.capture = None;
		self.source = None;
	}


	fn prepare_source( &mut self, geometry: &MonitorGeometry ) {
		if self.source.is_none() { self.source = unsafe { FindWindowW( w!( "Progman" ), PCWSTR::null() ) }.ok(); }
		if self.blur_percent == 0 {
			self.capture = None;
			self.ensure_thumbnail();
			self.update_thumbnail( geometry );
			return;
		}
		self.release_thumbnail();
		if self.capture.as_ref().is_some_and( |capture| !capture.matches( geometry ) ) { self.capture = None; }
		if let Some( capture ) = &mut self.capture { capture.set_blur( self.blur_percent ); return; }
		let Some( source ) = self.source else { return; };
		let mut source_rect = RECT::default();
		if unsafe { GetWindowRect( source, &mut source_rect ) }.is_err() { return; }
		if let Ok( mut capture ) = DesktopCapture::create( self.hwnd, source_rect, geometry ) { capture.set_blur( self.blur_percent ); self.capture = Some( capture ); }
	}


	fn ensure_thumbnail( &mut self ) {
		if self.thumbnail.is_some() { return; }
		let source = unsafe { FindWindowW( w!( "Progman" ), PCWSTR::null() ) }.ok();
		if let Some( source ) = source {
			if let Ok( thumbnail ) = unsafe { DwmRegisterThumbnail( self.hwnd, source ) } { self.thumbnail = Some( thumbnail ); }
		}
		self.source = source;
	}


	fn update_thumbnail( &self, geometry: &MonitorGeometry ) {
		let Some( source ) = self.source else { return; };
		let Some( thumbnail ) = self.thumbnail else { return; };
		let mut source_window_rect = RECT::default();
		if unsafe { GetWindowRect( source, &mut source_window_rect ) }.is_err() { return; }
		let source_rect = RECT { left: geometry.work_rect.left - source_window_rect.left, top: geometry.work_rect.top - source_window_rect.top, right: geometry.work_rect.right - source_window_rect.left, bottom: geometry.work_rect.bottom - source_window_rect.top };
		let destination = RECT { left: 0, top: 0, right: geometry.work_width(), bottom: geometry.work_height() };
		let properties = DWM_THUMBNAIL_PROPERTIES { dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_RECTSOURCE | DWM_TNP_OPACITY | DWM_TNP_VISIBLE | DWM_TNP_SOURCECLIENTAREAONLY, rcDestination: destination, rcSource: source_rect, opacity: 255, fVisible: true.into(), fSourceClientAreaOnly: false.into() };
		unsafe { let _ = DwmUpdateThumbnailProperties( thumbnail, &properties ); }
	}


	fn release_thumbnail( &mut self ) {
		if let Some( thumbnail ) = self.thumbnail.take() { unsafe { let _ = DwmUnregisterThumbnail( thumbnail ); } }
	}

}


impl Drop for DesktopBackdrop {
	fn drop( &mut self ) {
		self.release_thumbnail();
		unsafe { let _ = DestroyWindow( self.hwnd ); }
	}
}


unsafe fn create_window() -> WindowsResult< HWND > {
	let module = unsafe { GetModuleHandleW( None )? };
	let instance = module_instance( module );
	let class = WNDCLASSW { lpfnWndProc: Some( backdrop_window_proc ), hInstance: instance, hbrBackground: HBRUSH( unsafe { GetStockObject( BLACK_BRUSH ) }.0 ), lpszClassName: w!( "EpStartBackdropWindow" ), ..Default::default() };
	if unsafe { RegisterClassW( &class ) } == 0 { return Err( windows::core::Error::from_thread() ); }
	let hwnd = unsafe { CreateWindowExW( WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, w!( "EpStartBackdropWindow" ), w!( "ep_start_backdrop" ), WS_POPUP, 0, 0, 0, 0, None, None, Some( instance ), None )? };
	Ok( hwnd )
}


fn module_instance( module: HMODULE ) -> HINSTANCE {
	HINSTANCE( module.0 )
}


unsafe extern "system" fn backdrop_window_proc( hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	unsafe { DefWindowProcW( hwnd, message, wparam, lparam ) }
}
