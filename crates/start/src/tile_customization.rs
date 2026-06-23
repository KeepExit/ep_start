//! ::  Project Path  ->  ep_start :: tile_customization.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/23 02:28 周二


use std::env;
use std::ffi::OsStr;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{ Path, PathBuf };
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{ CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree, IPersistFile };
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{ FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FileOpenDialog, IFileOpenDialog, IShellLinkW, SIGDN_FILESYSPATH, ShellLink };
use windows::core::{ Interface, PCWSTR, w };


pub struct CreatedProgramTile {
	pub title: String,
	pub shortcut: PathBuf,
	pub icon_source: PathBuf,
}


pub fn choose_program( owner: HWND ) -> Result< Option< CreatedProgramTile >, String > {
	let selected = match choose_program_path( owner ) {
		Ok( selected ) => selected,
		Err( error ) if error.code().0 as u32 == 0x800704C7 => return Ok( None ),
		Err( error ) => return Err( format!( "选择程序失败：{}", error ) ),
	};
	let Some( selected ) = selected else { return Ok( None ); };
	let directory = poster_directory()?;
	fs::create_dir_all( &directory ).map_err( |error| format!( "创建磁贴快捷方式目录失败：{}", error ) )?;
	let title = selected.file_stem().and_then( OsStr::to_str ).filter( |title| !title.is_empty() ).unwrap_or( "Program" ).to_string();
	let destination = unique_shortcut_path( &directory, &title );
	let selected_shortcut = selected.extension().and_then( OsStr::to_str ).is_some_and( |extension| extension.eq_ignore_ascii_case( "lnk" ) );
	if selected_shortcut {
		fs::copy( &selected, &destination ).map_err( |error| format!( "复制快捷方式失败：{}", error ) )?;
	} else {
		create_shortcut( &selected, &destination )?;
	}
	let icon_source = if selected_shortcut { destination.clone() } else { selected };
	Ok( Some( CreatedProgramTile { title, shortcut: destination, icon_source } ) )
}


fn choose_program_path( owner: HWND ) -> windows::core::Result< Option< PathBuf > > {
	unsafe {
		let dialog: IFileOpenDialog = CoCreateInstance( &FileOpenDialog, None, CLSCTX_INPROC_SERVER )?;
		let options = dialog.GetOptions()?;
		dialog.SetOptions( options | FOS_FILEMUSTEXIST | FOS_FORCEFILESYSTEM )?;
		let filters = [ COMDLG_FILTERSPEC { pszName: w!( "Programs and shortcuts" ), pszSpec: w!( "*.exe;*.lnk" ) }, COMDLG_FILTERSPEC { pszName: w!( "All files" ), pszSpec: w!( "*.*" ) } ];
		dialog.SetFileTypes( &filters )?;
		dialog.Show( Some( owner ) )?;
		let item = dialog.GetResult()?;
		let path = item.GetDisplayName( SIGDN_FILESYSPATH )?;
		let value = path.to_string().ok().map( PathBuf::from );
		CoTaskMemFree( Some( path.0.cast() ) );
		Ok( value )
	}
}


fn create_shortcut( source: &Path, destination: &Path ) -> Result< (), String > {
	let source_wide = wide_null( source.as_os_str() );
	let destination_wide = wide_null( destination.as_os_str() );
	let working_directory = source.parent().map( |path| wide_null( path.as_os_str() ) );
	unsafe {
		let link: IShellLinkW = CoCreateInstance( &ShellLink, None, CLSCTX_INPROC_SERVER ).map_err( |error| format!( "创建 ShellLink 失败：{}", error ) )?;
		link.SetPath( PCWSTR( source_wide.as_ptr() ) ).map_err( |error| format!( "设置快捷方式目标失败：{}", error ) )?;
		if let Some( working_directory ) = &working_directory { link.SetWorkingDirectory( PCWSTR( working_directory.as_ptr() ) ).map_err( |error| format!( "设置快捷方式工作目录失败：{}", error ) )?; }
		let persist: IPersistFile = link.cast().map_err( |error| format!( "读取快捷方式保存接口失败：{}", error ) )?;
		persist.Save( PCWSTR( destination_wide.as_ptr() ), true ).map_err( |error| format!( "保存快捷方式失败：{}", error ) )?;
	}
	Ok( () )
}


fn poster_directory() -> Result< PathBuf, String > {
	let root = env::var_os( "LOCALAPPDATA" ).map( PathBuf::from ).ok_or_else( || "找不到 LOCALAPPDATA 用户数据目录".to_string() )?;
	Ok( root.join( "ep_start" ).join( "poster" ) )
}


fn unique_shortcut_path( directory: &Path, title: &str ) -> PathBuf {
	let first = directory.join( format!( "{}.lnk", title ) );
	if !first.exists() { return first; }
	let mut suffix = 2;
	loop {
		let candidate = directory.join( format!( "{} {}.lnk", title, suffix ) );
		if !candidate.exists() { return candidate; }
		suffix += 1;
	}
}


fn wide_null( value: &OsStr ) -> Vec< u16 > {
	value.encode_wide().chain( [ 0 ] ).collect()
}
