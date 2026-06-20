//! ::  Project Path  ->  ep_start :: mod.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 15:35 周六


mod icon;
mod menu;
mod menu_window;
mod native_theme;


pub use icon::EmbeddedIcon;
use menu::{ MENU_TOP_SPACER, PopupMenu };
use menu_window::MenuWindowStyler;
use native_theme::apply_native_menu_theme;
use std::ffi::c_void;
use std::mem::size_of;
use windows::Win32::Foundation::{ HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{ DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODT_MENU };
use windows::Win32::UI::Shell::{ NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION, NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW };
use windows::Win32::UI::WindowsAndMessaging::{ CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, GWLP_USERDATA, GetCursorPos, GetWindowLongPtrW, PostMessageW, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, SetWindowLongPtrW, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx, WM_APP, WM_CONTEXTMENU, WM_DRAWITEM, WM_MEASUREITEM, WM_NCCREATE, WM_NCDESTROY, WM_NULL, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP };
use windows::core::{ Result as WindowsResult, w };


const TRAY_ICON_ID: u32 = 1;
const WM_TRAY_EVENT: u32 = WM_APP + 40;
const NIN_KEYSELECT: u32 = NIN_SELECT + 1;


pub struct TrayIcon {
	state: *mut TrayState,
}


pub struct TrayIconConfig {
	pub tooltip: String,
	pub small_icon: &'static [ u8 ],
	pub large_icon: &'static [ u8 ],
	pub menu: Vec< TrayMenuEntry >,
}


#[derive( Clone, Debug )]
pub enum TrayMenuEntry {
	Command { id: u16, label: String },
	Separator,
}


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub enum TrayEvent {
	Activate,
	Command( u16 ),
}


struct TrayState {
	hwnd: HWND,
	icon: EmbeddedIcon,
	tooltip: String,
	menu_entries: Vec< TrayMenuEntry >,
	active_menu: Option< PopupMenu >,
	handler: Box< dyn FnMut( TrayEvent ) >,
	taskbar_created_message: u32,
	icon_added: bool,
	menu_dark: bool,
}


impl TrayMenuEntry {
	pub fn command( id: u16, label: impl Into< String > ) -> Self {
		Self::Command { id, label: label.into() }
	}


	pub fn separator() -> Self {
		Self::Separator
	}
}


impl TrayIcon {
	pub fn create( config: TrayIconConfig, handler: impl FnMut( TrayEvent ) + 'static ) -> Result< Self, String > {
		let icon = EmbeddedIcon::load( config.small_icon, config.large_icon )?;
		let taskbar_created_message = unsafe { RegisterWindowMessageW( w!( "TaskbarCreated" ) ) };
		let state = Box::new( TrayState { hwnd: HWND::default(), icon, tooltip: config.tooltip, menu_entries: config.menu, active_menu: None, handler: Box::new( handler ), taskbar_created_message, icon_added: false, menu_dark: false } );
		let state_pointer = Box::into_raw( state );
		if let Err( error ) = unsafe { create_host_window( state_pointer ) } {
			unsafe { drop( Box::from_raw( state_pointer ) ); }
			return Err( format!( "创建托盘消息窗口失败：{}", error ) );
		}
		let state = unsafe { &mut *state_pointer };
		if !state.add_icon() {
			unsafe { let _ = DestroyWindow( state.hwnd ); drop( Box::from_raw( state_pointer ) ); }
			return Err( "添加通知区域图标失败".to_string() );
		}
		Ok( Self { state: state_pointer } )
	}
}


impl Drop for TrayIcon {
	fn drop( &mut self ) {
		unsafe {
			let state = &mut *self.state;
			state.remove_icon();
			if !state.hwnd.is_invalid() { let _ = DestroyWindow( state.hwnd ); }
			drop( Box::from_raw( self.state ) );
		}
	}
}


impl TrayState {
	fn add_icon( &mut self ) -> bool {
		let mut data = self.icon_data();
		if !unsafe { Shell_NotifyIconW( NIM_ADD, &data ) }.as_bool() { return false; }
		data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
		if !unsafe { Shell_NotifyIconW( NIM_SETVERSION, &data ) }.as_bool() {
			unsafe { let _ = Shell_NotifyIconW( NIM_DELETE, &data ); }
			return false;
		}
		self.icon_added = true;
		true
	}


	fn remove_icon( &mut self ) {
		if !self.icon_added { return; }
		unsafe { let _ = Shell_NotifyIconW( NIM_DELETE, &self.icon_data() ); }
		self.icon_added = false;
	}


	fn restore_icon( &mut self ) {
		self.icon_added = false;
		let _ = self.add_icon();
	}


	fn icon_data( &self ) -> NOTIFYICONDATAW {
		let mut data = NOTIFYICONDATAW { cbSize: size_of::< NOTIFYICONDATAW >() as u32, hWnd: self.hwnd, uID: TRAY_ICON_ID, uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP, uCallbackMessage: WM_TRAY_EVENT, hIcon: self.icon.handle(), ..Default::default() };
		let tooltip: Vec< u16 > = self.tooltip.encode_utf16().take( data.szTip.len() - 1 ).collect();
		data.szTip[ ..tooltip.len() ].copy_from_slice( &tooltip );
		data
	}


	fn show_menu( &mut self ) {
		let dark = apply_native_menu_theme();
		self.menu_dark = dark;
		let Ok( popup ) = PopupMenu::create( &self.menu_entries ) else { return; };
		self.active_menu = Some( popup );
		let mut point = POINT::default();
		unsafe {
			let _ = GetCursorPos( &mut point );
			let _ = SetForegroundWindow( self.hwnd );
		}
		let _styler = MenuWindowStyler::install( dark );
		let command = unsafe { TrackPopupMenuEx( self.active_menu.as_ref().unwrap().handle(), ( TPM_RIGHTBUTTON | TPM_RETURNCMD ).0, point.x, point.y, self.hwnd, None ) }.0 as u16;
		self.active_menu = None;
		unsafe { let _ = PostMessageW( Some( self.hwnd ), WM_NULL, WPARAM( 0 ), LPARAM( 0 ) ); }
		if command != 0 { ( self.handler )( TrayEvent::Command( command ) ); }
	}


	fn measure_menu_item( &self, lparam: LPARAM ) -> bool {
		let measure = unsafe { &mut *( lparam.0 as *mut MEASUREITEMSTRUCT ) };
		if measure.CtlType != ODT_MENU || measure.itemData != MENU_TOP_SPACER { return false; }
		measure.itemWidth = 0;
		measure.itemHeight = 7;
		true
	}


	fn draw_menu_item( &self, lparam: LPARAM ) -> bool {
		let draw = unsafe { &*( lparam.0 as *const DRAWITEMSTRUCT ) };
		if draw.CtlType != ODT_MENU || draw.itemData != MENU_TOP_SPACER { return false; }
		let color = if self.menu_dark { windows::Win32::Foundation::COLORREF( 45 | 45 << 8 | 45 << 16 ) } else { windows::Win32::Foundation::COLORREF( 249 | 249 << 8 | 249 << 16 ) };
		let brush = unsafe { windows::Win32::Graphics::Gdi::CreateSolidBrush( color ) };
		unsafe { windows::Win32::Graphics::Gdi::FillRect( draw.hDC, &draw.rcItem, brush ); let _ = windows::Win32::Graphics::Gdi::DeleteObject( windows::Win32::Graphics::Gdi::HGDIOBJ( brush.0 ) ); }
		true
	}


	fn handle_tray_event( &mut self, lparam: LPARAM ) {
		match lparam.0 as u32 & 0xFFFF {
			NIN_SELECT | NIN_KEYSELECT => ( self.handler )( TrayEvent::Activate ),
			WM_CONTEXTMENU => self.show_menu(),
			_ => {}
		}
	}
}


unsafe fn create_host_window( state: *mut TrayState ) -> WindowsResult< () > {
	let module = unsafe { GetModuleHandleW( None )? };
	let instance = HINSTANCE( module.0 );
	let class = WNDCLASSW { lpfnWndProc: Some( tray_window_proc ), hInstance: instance, lpszClassName: w!( "EpStartTrayWindow" ), ..Default::default() };
	if unsafe { RegisterClassW( &class ) } == 0 { return Err( windows::core::Error::from_thread() ); }
	unsafe { CreateWindowExW( WS_EX_TOOLWINDOW, w!( "EpStartTrayWindow" ), w!( "ep_start tray" ), WS_POPUP, 0, 0, 0, 0, None, None, Some( instance ), Some( state.cast::< c_void >() ) )?; }
	Ok( () )
}


unsafe extern "system" fn tray_window_proc( hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if message == WM_NCCREATE {
		let creation = unsafe { &*( lparam.0 as *const CREATESTRUCTW ) };
		let state = creation.lpCreateParams.cast::< TrayState >();
		unsafe {
			( *state ).hwnd = hwnd;
			SetWindowLongPtrW( hwnd, GWLP_USERDATA, state as isize );
		}
	}
	let state = unsafe { GetWindowLongPtrW( hwnd, GWLP_USERDATA ) as *mut TrayState };
	if !state.is_null() {
		if message == unsafe { ( *state ).taskbar_created_message } { unsafe { ( *state ).restore_icon(); } return LRESULT( 0 ); }
		match message {
			WM_TRAY_EVENT => { unsafe { ( *state ).handle_tray_event( lparam ); } return LRESULT( 0 ); }
			WM_MEASUREITEM => { if unsafe { ( *state ).measure_menu_item( lparam ) } { return LRESULT( 1 ); } }
			WM_DRAWITEM => { if unsafe { ( *state ).draw_menu_item( lparam ) } { return LRESULT( 1 ); } }
			WM_NCDESTROY => { unsafe { SetWindowLongPtrW( hwnd, GWLP_USERDATA, 0 ); } }
			_ => {}
		}
	}
	unsafe { DefWindowProcW( hwnd, message, wparam, lparam ) }
}
