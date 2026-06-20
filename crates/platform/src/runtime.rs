//! ::  Project Path  ->  ep_start :: runtime.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 03:17 周六


use crate::input::GlobalInputManager;
use crate::message_loop;
use windows::Win32::System::Com::{ COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize };
use windows::Win32::UI::HiDpi::{ DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext };


pub struct PlatformRuntime {
	input: GlobalInputManager,
	_com: ComApartment,
}


impl PlatformRuntime {
	pub fn new() -> Result< Self, String > {
		unsafe { let _ = SetProcessDpiAwarenessContext( DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 ); }
		let com = ComApartment::initialize()?;
		let input = GlobalInputManager::new()?;
		Ok( Self { input, _com: com } )
	}


	pub fn input( &self ) -> &GlobalInputManager {
		&self.input
	}


	pub fn run( &self ) -> Result< (), String > {
		message_loop::run()
	}
}


struct ComApartment;


impl ComApartment {
	fn initialize() -> Result< Self, String > {
		unsafe { CoInitializeEx( None, COINIT_APARTMENTTHREADED ) }.ok().map_err( |error| format!( "初始化 COM 失败：{}", error ) )?;
		Ok( Self )
	}
}


impl Drop for ComApartment {
	fn drop( &mut self ) {
		unsafe { CoUninitialize(); }
	}
}
