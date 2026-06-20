//! ::  Project Path  ->  ep_start :: message_loop.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 03:17 周六


use windows::Win32::UI::WindowsAndMessaging::{ DispatchMessageW, GetMessageW, MSG, TranslateMessage };


pub fn run() -> Result< (), String > {
	let mut message = MSG::default();
	loop {
		let result = unsafe { GetMessageW( &mut message, None, 0, 0 ) };
		if result.0 == -1 { return Err( "Win32 消息循环读取失败".to_string() ); }
		if !result.as_bool() { break; }
		unsafe {
			let _ = TranslateMessage( &message );
			DispatchMessageW( &message );
		}
	}
	Ok( () )
}
