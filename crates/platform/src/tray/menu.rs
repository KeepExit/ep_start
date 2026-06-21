//! ::  Project Path  ->  ep_start :: menu.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 15:35 周六


use super::TrayMenuEntry;
use windows::Win32::UI::WindowsAndMessaging::{ AppendMenuW, CreatePopupMenu, DestroyMenu, HMENU, MF_SEPARATOR, MF_STRING };
use windows::core::PCWSTR;


pub struct PopupMenu {
	handle: HMENU,
}


impl PopupMenu {
	pub fn create( source: &[ TrayMenuEntry ] ) -> Result< Self, String > {
		let handle = unsafe { CreatePopupMenu() }.map_err( |error| format!( "创建托盘菜单失败：{}", error ) )?;
		let popup = Self { handle };
		for entry in source {
			let result = match entry {
				TrayMenuEntry::Command { id, label } => {
					let label: Vec< u16 > = label.encode_utf16().chain( [ 0 ] ).collect();
					unsafe { AppendMenuW( popup.handle, MF_STRING, *id as usize, PCWSTR( label.as_ptr() ) ) }
				}
				TrayMenuEntry::Separator => unsafe { AppendMenuW( popup.handle, MF_SEPARATOR, 0, PCWSTR::null() ) },
			};
			result.map_err( |error| format!( "添加托盘菜单项失败：{}", error ) )?;
		}
		Ok( popup )
	}


	pub fn handle( &self ) -> HMENU {
		self.handle
	}
}


impl Drop for PopupMenu {
	fn drop( &mut self ) {
		unsafe { let _ = DestroyMenu( self.handle ); }
	}
}
