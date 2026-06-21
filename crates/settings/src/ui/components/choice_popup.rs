//! ::  Project Path  ->  ep_start :: choice_popup.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 18:22 周日


use crate::ui::geometry::{ UiRect, scale };
use crate::ui::paint_buffer::paint_buffered;
use crate::ui::painter::Painter;
use crate::ui::theme::SettingsTheme;
use platform::MonitorGeometry;
use std::ffi::c_void;
use std::mem::size_of;
use std::sync::OnceLock;
use windows::Win32::Foundation::{ HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM };
use windows::Win32::Graphics::Dwm::{ DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUNDSMALL, DwmSetWindowAttribute };
use windows::Win32::Graphics::Gdi::{ BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{ ReleaseCapture, SetCapture, SetFocus, VK_DOWN, VK_ESCAPE, VK_RETURN, VK_UP };
use windows::Win32::UI::WindowsAndMessaging::{ CREATESTRUCTW, CS_DROPSHADOW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetClientRect, GetMessageW, GetWindowLongPtrW, IDC_ARROW, LoadCursorW, MSG, PostQuitMessage, RegisterClassW, SW_HIDE, SW_SHOW, SetForegroundWindow, SetWindowLongPtrW, ShowWindow, TranslateMessage, WM_CLOSE, WM_ERASEBKGND, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP };
use windows::core::w;


const POPUP_GAP: i32 = 4;
const POPUP_PADDING: i32 = 4;
const ITEM_HEIGHT: i32 = 36;
const ITEM_HORIZONTAL_MARGIN: i32 = 4;
const ITEM_TEXT_PADDING: i32 = 16;
const ACCENT_WIDTH: i32 = 4;
const ACCENT_HEIGHT: i32 = 20;


struct ChoicePopupState {
	hwnd: HWND,
	options: Vec< ChoiceOption >,
	selected: u8,
	hovered: Option< usize >,
	result: Option< u8 >,
	closed: bool,
	theme: SettingsTheme,
	dpi: i32,
}


#[derive( Clone )]
pub( super ) struct ChoiceOption {
	value: u8,
	label: String,
}


impl ChoiceOption {
	pub( super ) fn new( value: u8, label: String ) -> Self {
		Self { value, label }
	}

	pub( super ) fn number( value: u8 ) -> Self {
		Self::new( value, value.to_string() )
	}
}


pub( super ) fn show_choice_popup( owner: HWND, anchor: RECT, options: &[ ChoiceOption ], selected: u8, theme: SettingsTheme ) -> Option< u8 > {
	if options.is_empty() || !ensure_window_class() { return None; }
	let dpi = unsafe { GetDpiForWindow( owner ) }.max( 96 ) as i32;
	let width = ( anchor.right - anchor.left ).max( scale( 96, dpi ) );
	let height = scale( POPUP_PADDING * 2 + ITEM_HEIGHT * options.len() as i32, dpi );
	let geometry = MonitorGeometry::from_window( owner ).ok()?;
	let gap = scale( POPUP_GAP, dpi );
	let mut left = anchor.left;
	let mut top = if anchor.bottom + gap + height <= geometry.work_rect.bottom { anchor.bottom + gap } else { anchor.top - gap - height };
	left = left.clamp( geometry.work_rect.left, ( geometry.work_rect.right - width ).max( geometry.work_rect.left ) );
	top = top.clamp( geometry.work_rect.top, ( geometry.work_rect.bottom - height ).max( geometry.work_rect.top ) );
	let state = Box::into_raw( Box::new( ChoicePopupState { hwnd: HWND::default(), options: options.to_vec(), selected, hovered: options.iter().position( |option| option.value == selected ), result: None, closed: false, theme, dpi } ) );
	let Some( hwnd ) = ( unsafe { create_window( owner, state, left, top, width, height ) } ) else { unsafe { drop( Box::from_raw( state ) ); } return None; };
	unsafe {
		( *state ).hwnd = hwnd;
		apply_window_style( hwnd, theme.dark );
		let _ = ShowWindow( hwnd, SW_SHOW );
		let _ = SetForegroundWindow( hwnd );
		let _ = SetFocus( Some( hwnd ) );
		SetCapture( hwnd );
		let _ = InvalidateRect( Some( hwnd ), None, false );
	}
	let mut quit_code = None;
	let mut message = MSG::default();
	while unsafe { !( *state ).closed } {
		let status = unsafe { GetMessageW( &mut message, None, 0, 0 ) }.0;
		if status <= 0 {
			if status == 0 { quit_code = Some( message.wParam.0 as i32 ); }
			break;
		}
		unsafe {
			let _ = TranslateMessage( &message );
			DispatchMessageW( &message );
		}
	}
	unsafe {
		let _ = ReleaseCapture();
		let _ = ShowWindow( hwnd, SW_HIDE );
		let _ = DestroyWindow( hwnd );
	}
	let state = unsafe { Box::from_raw( state ) };
	let result = state.result;
	if let Some( code ) = quit_code { unsafe { PostQuitMessage( code ); } }
	result
}


impl ChoicePopupState {
	fn paint( &self, hdc: windows::Win32::Graphics::Gdi::HDC, client: RECT ) {
		paint_buffered( hdc, client, |buffer| {
			let painter = Painter::new( buffer, self.dpi );
			painter.fill( client, self.theme.card );
			for ( index, option ) in self.options.iter().enumerate() {
				let area = self.item_area( client, index );
				if self.hovered == Some( index ) || option.value == self.selected { painter.round_rect( area, 4, self.theme.track ); }
				if option.value == self.selected {
					let accent_height = scale( ACCENT_HEIGHT, self.dpi );
					let accent_top = area.center_y() - accent_height / 2;
					painter.round_rect( UiRect::new( area.left, accent_top, area.left + scale( ACCENT_WIDTH, self.dpi ), accent_top + accent_height ), 3, self.theme.accent );
				}
				painter.text( &option.label, UiRect::new( area.left + scale( ITEM_TEXT_PADDING, self.dpi ), area.top, area.right - scale( ITEM_TEXT_PADDING, self.dpi ), area.bottom ), 14, windows::Win32::Graphics::Gdi::FW_NORMAL.0 as i32, self.theme.text );
			}
		} );
	}

	fn item_area( &self, client: RECT, index: usize ) -> UiRect {
		let margin = scale( ITEM_HORIZONTAL_MARGIN, self.dpi );
		let top = scale( POPUP_PADDING + ITEM_HEIGHT * index as i32, self.dpi );
		UiRect::new( client.left + margin, top, client.right - margin, top + scale( ITEM_HEIGHT, self.dpi ) )
	}

	fn item_at( &self, x: i32, y: i32 ) -> Option< usize > {
		let mut client = RECT::default();
		unsafe { let _ = GetClientRect( self.hwnd, &mut client ); }
		self.options.iter().enumerate().find( |( index, _ )| self.item_area( client, *index ).contains( x, y ) ).map( |( index, _ )| index )
	}

	fn close( &mut self, result: Option< u8 > ) {
		if self.closed { return; }
		self.result = result;
		self.closed = true;
		unsafe {
			let _ = ReleaseCapture();
			let _ = ShowWindow( self.hwnd, SW_HIDE );
		}
	}

	fn move_selection( &mut self, offset: isize ) {
		let current = self.hovered.or_else( || self.options.iter().position( |option| option.value == self.selected ) ).unwrap_or( 0 ) as isize;
		let next = ( current + offset ).clamp( 0, self.options.len().saturating_sub( 1 ) as isize ) as usize;
		if self.hovered != Some( next ) { self.hovered = Some( next ); unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); } }
	}
}


fn ensure_window_class() -> bool {
	static CLASS_ATOM: OnceLock< u16 > = OnceLock::new();
	*CLASS_ATOM.get_or_init( || {
		let Ok( module ) = ( unsafe { GetModuleHandleW( None ) } ) else { return 0; };
		let class = WNDCLASSW { style: CS_DROPSHADOW, lpfnWndProc: Some( choice_popup_proc ), hInstance: HINSTANCE( module.0 ), hCursor: unsafe { LoadCursorW( None, IDC_ARROW ).unwrap_or_default() }, lpszClassName: w!( "EpStartSettingsChoicePopup" ), ..Default::default() };
		unsafe { RegisterClassW( &class ) }
	} ) != 0
}


unsafe fn create_window( owner: HWND, state: *mut ChoicePopupState, x: i32, y: i32, width: i32, height: i32 ) -> Option< HWND > {
	let module = unsafe { GetModuleHandleW( None ).ok()? };
	unsafe { CreateWindowExW( WS_EX_TOOLWINDOW | WS_EX_TOPMOST, w!( "EpStartSettingsChoicePopup" ), w!( "" ), WS_POPUP, x, y, width, height, Some( owner ), None, Some( HINSTANCE( module.0 ) ), Some( state.cast::< c_void >() ) ).ok() }
}


fn apply_window_style( hwnd: HWND, dark: bool ) {
	let corner = DWMWCP_ROUNDSMALL;
	let dark = dark as i32;
	unsafe {
		let _ = DwmSetWindowAttribute( hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, std::ptr::from_ref( &corner ).cast(), size_of_val( &corner ) as u32 );
		let _ = DwmSetWindowAttribute( hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, std::ptr::from_ref( &dark ).cast(), size_of::< i32 >() as u32 );
	}
}


unsafe extern "system" fn choice_popup_proc( hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if message == WM_NCCREATE {
		let creation = unsafe { &*( lparam.0 as *const CREATESTRUCTW ) };
		unsafe { SetWindowLongPtrW( hwnd, GWLP_USERDATA, creation.lpCreateParams as isize ); }
	}
	let state = unsafe { GetWindowLongPtrW( hwnd, GWLP_USERDATA ) as *mut ChoicePopupState };
	if !state.is_null() {
		match message {
			WM_PAINT => {
				let mut paint = PAINTSTRUCT::default();
				let mut client = RECT::default();
				unsafe {
					BeginPaint( hwnd, &mut paint );
					let _ = GetClientRect( hwnd, &mut client );
					( *state ).paint( paint.hdc, client );
					let _ = EndPaint( hwnd, &paint );
				}
				return LRESULT( 0 );
			}
			WM_MOUSEMOVE => {
				let x = lparam.0 as i16 as i32;
				let y = ( lparam.0 >> 16 ) as i16 as i32;
				let hovered = unsafe { ( *state ).item_at( x, y ) };
				if unsafe { ( *state ).hovered } != hovered { unsafe { ( *state ).hovered = hovered; let _ = InvalidateRect( Some( hwnd ), None, false ); } }
				return LRESULT( 0 );
			}
			WM_LBUTTONUP => {
				let x = lparam.0 as i16 as i32;
				let y = ( lparam.0 >> 16 ) as i16 as i32;
				unsafe {
					let state = &mut *state;
					let result = state.item_at( x, y ).map( |index| state.options[ index ].value );
					state.close( result );
				}
				return LRESULT( 0 );
			}
			WM_KEYDOWN => {
				match wparam.0 as u16 {
					key if key == VK_ESCAPE.0 => unsafe { ( *state ).close( None ); },
					key if key == VK_UP.0 => unsafe { ( *state ).move_selection( -1 ); },
					key if key == VK_DOWN.0 => unsafe { ( *state ).move_selection( 1 ); },
					key if key == VK_RETURN.0 => {
						unsafe {
							let state = &mut *state;
							let result = state.hovered.map( |index| state.options[ index ].value );
							state.close( result );
						}
					}
					_ => {}
				}
				return LRESULT( 0 );
			}
			WM_KILLFOCUS | WM_CLOSE => { unsafe { ( *state ).close( None ); } return LRESULT( 0 ); }
			WM_ERASEBKGND => { return LRESULT( 1 ); }
			WM_NCDESTROY => { unsafe { SetWindowLongPtrW( hwnd, GWLP_USERDATA, 0 ); } }
			_ => {}
		}
	}
	unsafe { DefWindowProcW( hwnd, message, wparam, lparam ) }
}
