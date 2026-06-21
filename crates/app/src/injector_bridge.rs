//! ::  Project Path  ->  ep_start :: injector_bridge.rs :: injector_bridge
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/22 01:52 周一


use std::process::Command;


pub fn ensure_injected() {
	let result = Command::new( "injector.exe" ).output();

	match result {
		Ok( _ ) => {}
		Err( e ) => {
			println!( "injector 启动失败: {:?}", e );
		}
	}
}