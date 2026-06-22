//! ::  Project Path  ->  ep_start :: backdrop.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 03:17 周六


use platform::MonitorGeometry;
use crate::backdrop_capture::DesktopCapture;
use std::ffi::c_void;
use std::sync::mpsc::{ Receiver, TryRecvError, sync_channel };
use std::thread;
use windows::Win32::Foundation::{ HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM };
use windows::Win32::Graphics::Dwm::{ DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION, DWM_TNP_RECTSOURCE, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE, DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties };
use windows::Win32::Graphics::Gdi::{ BLACK_BRUSH, GetStockObject, HBRUSH };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::WinRT::{ RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize };
use windows::Win32::UI::WindowsAndMessaging::{ CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW, GetWindowRect, HWND_TOPMOST, PostMessageW, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetWindowPos, ShowWindow, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP };
use windows::core::{ PCWSTR, Result as WindowsResult, w };


pub struct DesktopBackdrop {
	hwnd: HWND,
	source: Option< HWND >,
	thumbnail: Option< isize >,
	capture: Option< DesktopCapture >,
	capture_receiver: Option< Receiver< Result< DesktopCapture, String > > >,
	capture_request_geometry: Option< MonitorGeometry >,
	blur_percent: u8,
	notify_hwnd: HWND,
	notify_message: u32,
}


impl DesktopBackdrop {
	pub fn create() -> Result< Self, String > {
		let hwnd = unsafe { create_window() }.map_err( |error| format!( "创建桌面背景窗口失败：{}", error ) )?;
		Ok( Self { hwnd, source: None, thumbnail: None, capture: None, capture_receiver: None, capture_request_geometry: None, blur_percent: 0, notify_hwnd: HWND::default(), notify_message: 0 } )
	}


	pub fn show( &mut self, geometry: &MonitorGeometry, blur_percent: u8, notify_hwnd: HWND, notify_message: u32 ) {
		self.blur_percent = blur_percent.min( 100 );
		self.notify_hwnd = notify_hwnd;
		self.notify_message = notify_message;
		self.prepare_source( geometry );
		unsafe {
			let _ = SetWindowPos( self.hwnd, Some( HWND_TOPMOST ), geometry.work_rect.left, geometry.work_rect.top, geometry.work_width(), geometry.work_height(), SWP_NOACTIVATE | SWP_SHOWWINDOW );
			let _ = ShowWindow( self.hwnd, SW_SHOWNOACTIVATE );
		}
	}


	pub fn set_blur( &mut self, geometry: &MonitorGeometry, blur_percent: u8 ) {
		self.blur_percent = blur_percent.min( 100 );
		self.prepare_source( geometry );
	}


	pub fn update_frame( &mut self, geometry: &MonitorGeometry ) -> bool {
		self.receive_capture( geometry );
		let Some( capture ) = &mut self.capture else { return false; };
		capture.set_blur( self.blur_percent );
		let presented = capture.present_next_frame();
		if presented { self.release_thumbnail(); }
		presented
	}


	pub fn hide( &mut self ) {
		unsafe { let _ = ShowWindow( self.hwnd, SW_HIDE ); }
		self.release_thumbnail();
		self.capture = None;
		self.capture_receiver = None;
		self.capture_request_geometry = None;
		self.source = None;
	}


	fn prepare_source( &mut self, geometry: &MonitorGeometry ) {
		if self.source.is_none() { self.source = unsafe { FindWindowW( w!( "Progman" ), PCWSTR::null() ) }.ok(); }
		if self.blur_percent == 0 {
			self.capture = None;
			self.capture_receiver = None;
			self.capture_request_geometry = None;
			self.ensure_thumbnail();
			self.update_thumbnail( geometry );
			return;
		}
		if self.capture.as_ref().is_some_and( |capture| !capture.matches( geometry ) ) { self.capture = None; }
		if let Some( capture ) = &mut self.capture { capture.set_blur( self.blur_percent ); return; }
		self.ensure_thumbnail();
		self.update_thumbnail( geometry );
		if self.capture_request_geometry.as_ref() == Some( geometry ) && self.capture_receiver.is_some() { return; }
		self.capture_receiver = None;
		self.capture_request_geometry = None;
		let Some( source ) = self.source else { return; };
		let mut source_rect = RECT::default();
		if unsafe { GetWindowRect( source, &mut source_rect ) }.is_err() { return; }
		self.begin_capture( source_rect, *geometry );
	}


	fn begin_capture( &mut self, source_rect: RECT, geometry: MonitorGeometry ) {
		if self.notify_hwnd.is_invalid() || self.notify_message == 0 { return; }
		let ( sender, receiver ) = sync_channel( 1 );
		let backdrop_handle = self.hwnd.0 as isize;
		let notify_handle = self.notify_hwnd.0 as isize;
		let notify_message = self.notify_message;
		let work_rect = geometry.work_rect;
		self.capture_receiver = Some( receiver );
		self.capture_request_geometry = Some( geometry );
		thread::spawn( move || {
			let backdrop_hwnd = HWND( backdrop_handle as *mut c_void );
			let notify_hwnd = HWND( notify_handle as *mut c_void );
			let geometry = MonitorGeometry { work_rect, ..Default::default() };
			let initialized = unsafe { RoInitialize( RO_INIT_MULTITHREADED ) }.is_ok();
			let result = DesktopCapture::create( backdrop_hwnd, source_rect, &geometry, notify_hwnd, notify_message );
			let _ = sender.send( result );
			unsafe { let _ = PostMessageW( Some( notify_hwnd ), notify_message, WPARAM( 0 ), LPARAM( 0 ) ); }
			if initialized { unsafe { RoUninitialize(); } }
		} );
	}


	fn receive_capture( &mut self, geometry: &MonitorGeometry ) {
		let Some( receiver ) = &self.capture_receiver else { return; };
		let result = match receiver.try_recv() {
			Ok( result ) => Some( result ),
			Err( TryRecvError::Empty ) => None,
			Err( TryRecvError::Disconnected ) => Some( Err( "实时背景初始化线程已结束".to_string() ) ),
		};
		let Some( result ) = result else { return; };
		self.capture_receiver = None;
		self.capture_request_geometry = None;
		if self.blur_percent == 0 { return; }
		if let Ok( capture ) = result {
			if !capture.matches( geometry ) { return; }
			self.capture = Some( capture );
		}
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
	let hwnd = unsafe { CreateWindowExW( WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST, w!( "EpStartBackdropWindow" ), w!( "ep_start_backdrop" ), WS_POPUP, 0, 0, 0, 0, None, None, Some( instance ), None )? };
	Ok( hwnd )
}


fn module_instance( module: HMODULE ) -> HINSTANCE {
	HINSTANCE( module.0 )
}


unsafe extern "system" fn backdrop_window_proc( hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	unsafe { DefWindowProcW( hwnd, message, wparam, lparam ) }
}
