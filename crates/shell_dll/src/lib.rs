//! ::  Project Path  ->  ep_start :: lib.rs :: shell_dll
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 22:42 周日


use windows::core::{ w, BOOL };
use windows::Win32::Foundation::*;
use windows::Win32::System::SystemServices::*;
use windows::Win32::UI::WindowsAndMessaging::*;


#[unsafe( no_mangle )]
pub extern "system" fn DllMain( _hinst: HINSTANCE, reason: u32, _: *mut std::ffi::c_void ) -> BOOL {
	match reason {
		DLL_PROCESS_ATTACH => {
			unsafe {
				MessageBoxW(
					None,
					w!( "Injected into Explorer!" ),
					w!( "ep_start" ),
					MB_OK
				);
			}
		}
		_ => {}
	}

	BOOL( 1 )
}