//! ::  Project Path  ->  ep_start :: main.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/22 01:21 周一


use std::ffi::CString;
use windows::core::{ PCSTR, w };
use windows::Win32::System::Diagnostics::ToolHelp::*;
use windows::Win32::System::Threading::*;
use windows::Win32::System::Memory::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::System::Diagnostics::Debug::*;
use windows::Win32::UI::WindowsAndMessaging::*;


fn main() {
	let pid = find_explorer();

	if is_injected( pid ) {
		println!( "Already injected" );
		return;
	}

	inject( pid );

	println!( "Injected into explorer.exe" );
}


fn find_explorer() -> u32 {
	let mut pid = 0;
	unsafe {
		if let Ok( hwnd ) = FindWindowW( w!( "Shell_TrayWnd" ), None ) {
			if hwnd.is_invalid() {
				panic!( "找不到任务栏" );
			}
			GetWindowThreadProcessId( hwnd, Some( &mut pid ) );
		}
	}
	pid
}


fn is_injected( pid: u32 ) -> bool {
	unsafe {
		let snapshot = CreateToolhelp32Snapshot(
			TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32,
			pid
		);

		if snapshot.is_err() { return false; }

		let snapshot = snapshot.unwrap();

		let mut entry = MODULEENTRY32::default();
		entry.dwSize = size_of::< MODULEENTRY32 >() as u32;

		if( Module32First( snapshot, &mut entry ).is_ok() ) {
			loop {
				let raw = &entry.szModule;
				let len = raw.iter().position( |&c| c == 0 ).unwrap_or( raw.len() );
				let bytes = raw[ ..len ].iter().map( |c| *c as u8 ).collect::< Vec< u8 > >();
				let name = String::from_utf8_lossy( &bytes );
				if name.contains( "ep_shell.dll" ) { return true; }

				if Module32Next( snapshot, &mut entry ).is_err() { break; }
			}
		}
	}
	false
}


fn inject( pid: u32 ) {
	let process = unsafe {
		OpenProcess( PROCESS_ALL_ACCESS, false, pid ).unwrap()
	};

	let dll_path = CString::new( "C:\\ep_shell.dll" ).unwrap();

	let remote_mem = unsafe {
		VirtualAllocEx(
			process,
			None,
			dll_path.as_bytes_with_nul().len(),
			MEM_COMMIT | MEM_RESERVE,
			PAGE_READWRITE
		)
	};

	if( remote_mem.is_null() ) {
		panic!( "VirtualAllocEx failed" );
	}

	unsafe {
		WriteProcessMemory(
			process,
			remote_mem,
			dll_path.as_ptr() as _,
			dll_path.as_bytes_with_nul().len(),
			None
		).unwrap();
	}

	let kernel32 = unsafe {
		GetModuleHandleA( PCSTR( b"kernel32.dll\0".as_ptr() ) ).unwrap()
	};

	let load_lib = unsafe {
		GetProcAddress(
			kernel32,
			PCSTR( b"LoadLibraryA\0".as_ptr() )
		)
	}.unwrap();

	unsafe {
		CreateRemoteThread( process, None, 0, Some( std::mem::transmute( load_lib ) ), Some( remote_mem ), 0, None ).unwrap();
	}
}