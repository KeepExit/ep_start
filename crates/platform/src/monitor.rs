//! ::  Project Path  ->  ep_start :: monitor.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 03:17 周六


use std::mem::size_of;
use windows::Win32::Foundation::{ HWND, POINT, RECT };
use windows::Win32::Graphics::Gdi::{ GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, MonitorFromWindow };
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;


#[derive( Clone, Copy, Debug, Default, PartialEq )]
pub struct MonitorGeometry {
	pub monitor: HMONITOR,
	pub monitor_rect: RECT,
	pub work_rect: RECT,
}


impl MonitorGeometry {
	pub fn from_cursor() -> Result< Self, String > {
		let mut cursor = POINT::default();
		unsafe { GetCursorPos( &mut cursor ) }.map_err( |error| format!( "读取鼠标所在显示器失败：{}", error ) )?;
		let monitor = unsafe { MonitorFromPoint( cursor, MONITOR_DEFAULTTONEAREST ) };
		Self::from_monitor( monitor )
	}


	pub fn from_window( hwnd: HWND ) -> Result< Self, String > {
		let monitor = unsafe { MonitorFromWindow( hwnd, MONITOR_DEFAULTTONEAREST ) };
		Self::from_monitor( monitor )
	}


	pub fn work_width( &self ) -> i32 {
		self.work_rect.right - self.work_rect.left
	}


	pub fn work_height( &self ) -> i32 {
		self.work_rect.bottom - self.work_rect.top
	}


	fn from_monitor( monitor: windows::Win32::Graphics::Gdi::HMONITOR ) -> Result< Self, String > {
		let mut information = MONITORINFO { cbSize: size_of::< MONITORINFO >() as u32, ..Default::default() };
		if !unsafe { GetMonitorInfoW( monitor, &mut information ) }.as_bool() { return Err( format!( "读取显示器工作区失败：{}", windows::core::Error::from_thread() ) ); }
		Ok( Self { monitor, monitor_rect: information.rcMonitor, work_rect: information.rcWork } )
	}
}
