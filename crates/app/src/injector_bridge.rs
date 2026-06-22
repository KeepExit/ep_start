//! ::  Project Path  ->  ep_start :: injector_bridge.rs :: injector_bridge
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/22 01:52 周一


use crate_injector::ShellInjection;
use shell_dll as _;
use std::env;
use std::path::PathBuf;


pub fn install() -> Result< ShellInjection, String > {
	let dll = locate_shell_dll()?;
	ShellInjection::install( &dll )
}


fn locate_shell_dll() -> Result< PathBuf, String > {
	if let Some( configured ) = env::var_os( "EP_START_SHELL_DLL" ).map( PathBuf::from ) {
		if configured.is_file() { return Ok( configured ); }
		return Err( format!( "EP_START_SHELL_DLL 指向的文件不存在：{}", configured.display() ) );
	}
	let executable = env::current_exe().map_err( |error| format!( "读取应用程序路径失败：{}", error ) )?;
	let directory = executable.parent().ok_or_else( || "应用程序路径没有父目录".to_string() )?;
	let candidates = [
		directory.join( "shell_dll.dll" ),
		directory.join( "deps" ).join( "shell_dll.dll" ),
		directory.join( "ep_shell.dll" ),
	];
	candidates.into_iter().find( |candidate| candidate.is_file() ).ok_or_else( || format!( "找不到 Explorer 桥接 DLL；已检查 {} 和其 deps 目录", directory.display() ) )
}
