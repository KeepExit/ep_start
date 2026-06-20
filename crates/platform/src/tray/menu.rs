//! ::  Project Path  ->  ep_start :: menu.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 15:35 周六


use super::TrayMenuEntry;
use std::mem::size_of;
use windows::Win32::UI::WindowsAndMessaging::{ AppendMenuW, CreatePopupMenu, DestroyMenu, HMENU, InsertMenuItemW, MENUITEMINFOW, MFS_DISABLED, MFT_OWNERDRAW, MF_SEPARATOR, MF_STRING, MIIM_DATA, MIIM_FTYPE, MIIM_STATE };
use windows::core::PCWSTR;


pub struct PopupMenu {
	handle: HMENU,
}


pub const MENU_TOP_SPACER: usize = 0x4550_5350;


impl PopupMenu {
	pub fn create( source: &[ TrayMenuEntry ] ) -> Result< Self, String > {
		let handle = unsafe { CreatePopupMenu() }.map_err( |error| format!( "创建托盘菜单失败：{}", error ) )?;
		let popup = Self { handle };
		let spacer = MENUITEMINFOW { cbSize: size_of::< MENUITEMINFOW >() as u32, fMask: MIIM_FTYPE | MIIM_STATE | MIIM_DATA, fType: MFT_OWNERDRAW, fState: MFS_DISABLED, dwItemData: MENU_TOP_SPACER, ..Default::default() };
		unsafe { InsertMenuItemW( popup.handle, 0, true, &spacer ) }.map_err( |error| format!( "添加托盘菜单顶部间距失败：{}", error ) )?;
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
