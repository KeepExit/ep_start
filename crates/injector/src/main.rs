//! ::  Project Path  ->  ep_start :: main.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/22 01:21 周一


use injector::ShellInjection;
use std::env;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;


fn main() {
	let Some( dll_path ) = env::args_os().nth( 1 ).map( PathBuf::from ) else { eprintln!( "用法：injector.exe <shell_dll.dll>" ); return; };
	match ShellInjection::install( &dll_path ) {
		Ok( _injection ) => { thread::sleep( Duration::from_secs( 1 ) ); println!( "Explorer 桥接已安装" ); }
		Err( error ) => eprintln!( "{}", error ),
	}
}
