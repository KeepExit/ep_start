//! ::  Project Path  ->  ep_start :: process.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 20:46 周六


use windows::Win32::System::Threading::{ GetCurrentProcess, SetProcessWorkingSetSize };


pub fn trim_working_set() {
	unsafe { let _ = SetProcessWorkingSetSize( GetCurrentProcess(), usize::MAX, usize::MAX ); }
}
