//! ::  Project Path  ->  ep_start :: window.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 20:46 周六


use crate::ui::layout::{ SettingId, SettingsLayout, ControlKind, scale };
use crate::ui::components::{ SettingView, choose_choice_value, draw_action_button, draw_setting_row, draw_sidebar_item, slider_ratio_from_x };
use crate::ui::paint_buffer::paint_buffered as draw_buffered;
use crate::ui::painter::Painter;
use crate::ui::theme::SettingsTheme;
use crate::window_state::{ WindowSize, WindowSizeStore };
use configuration::{ AppPreferences, ConfigurationStore, StartPreferences };
use localization::TextResources;
use platform::{ EmbeddedIcon, MonitorGeometry, show_error_dialog, trim_working_set };
use std::ffi::c_void;
use std::mem::size_of;
use windows::Win32::Foundation::{ HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM };
use windows::Win32::Graphics::Dwm::{ DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute };
use windows::Win32::Graphics::Gdi::{ BeginPaint, ClientToScreen, EndPaint, FW_SEMIBOLD, HDC, IntersectClipRect, InvalidateRect, PAINTSTRUCT, RestoreDC, SaveDC };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{ GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI };
use windows::Win32::UI::Input::KeyboardAndMouse::{ ReleaseCapture, SetCapture };
use windows::Win32::UI::WindowsAndMessaging::{ CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, GetWindowRect, ICON_SMALL, IDC_ARROW, LoadCursorW, MINMAXINFO, PostMessageW, RegisterClassW, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SendMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_APP, WM_CLOSE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_EXITSIZEMOVE, WM_GETMINMAXINFO, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SETICON, WM_SETTINGCHANGE, WM_SIZE, WNDCLASSW, WS_OVERLAPPEDWINDOW };
use windows::core::{ Result as WindowsResult, w };


const WM_SHOW_SETTINGS: u32 = WM_APP + 50;

pub struct SettingsRuntime {
	window: SettingsWindow,
}

#[derive( Clone, Copy )]
pub struct SettingsController {
	hwnd: HWND,
}

struct SettingsWindow {
	hwnd: HWND,
	state: *mut SettingsState,
	_large_icon: EmbeddedIcon,
	_small_icon: EmbeddedIcon,
}

struct SettingsState {
	hwnd: HWND,
	store: ConfigurationStore,
	saved_preferences: AppPreferences,
	draft_preferences: AppPreferences,
	text: TextResources,
	theme: SettingsTheme,
	pointer_drag: Option< PointerDrag >,
	scroll_y: i32,
	on_change: Box< dyn FnMut( StartPreferences ) >,
}

#[derive( Clone, Copy )]
enum PointerDrag {
	Slider( SettingId ),
	Scrollbar( i32 ),
}

impl SettingsRuntime {
	pub fn new( store: ConfigurationStore, preferences: AppPreferences, small_icon: &'static [ u8 ], large_icon: &'static [ u8 ], on_change: impl FnMut( StartPreferences ) + 'static ) -> Result< Self, String > {
		Ok( Self { window: SettingsWindow::create( store, preferences, small_icon, large_icon, on_change )? } )
	}
	pub fn controller( &self ) -> SettingsController {
		SettingsController { hwnd: self.window.hwnd }
	}
}

impl SettingsController {
	pub fn show( &self ) {
		unsafe { let _ = PostMessageW( Some( self.hwnd ), WM_SHOW_SETTINGS, WPARAM( 0 ), LPARAM( 0 ) ); }
	}
}

impl SettingsWindow {
	fn create( store: ConfigurationStore, preferences: AppPreferences, small_icon_source: &'static [ u8 ], large_icon_source: &'static [ u8 ], on_change: impl FnMut( StartPreferences ) + 'static ) -> Result< Self, String > {
		let large_icon = EmbeddedIcon::load_for_size( large_icon_source, 32, 32 )?;
		let small_icon = EmbeddedIcon::load_for_size( small_icon_source, 16, 16 )?;
		let text = TextResources::system()?;
		let state = Box::into_raw( Box::new( SettingsState { hwnd: HWND::default(), store, saved_preferences: preferences, draft_preferences: preferences, text, theme: SettingsTheme::system(), pointer_drag: None, scroll_y: 0, on_change: Box::new( on_change ) } ) );
		let hwnd = match unsafe { create_window( state, large_icon.handle(), small_icon.handle() ) } {
			Ok( hwnd ) => hwnd,
			Err( error ) => {
				unsafe { drop( Box::from_raw( state ) ); }
				return Err( format!( "创建设置窗口失败：{}", error ) );
			}
		};
		Ok( Self { hwnd, state, _large_icon: large_icon, _small_icon: small_icon } )
	}
}

impl Drop for SettingsWindow {
	fn drop( &mut self ) {
		unsafe {
			let _ = DestroyWindow( self.hwnd );
			drop( Box::from_raw( self.state ) );
		}
	}
}

impl SettingsState {
	fn show( &mut self ) {
		let Ok( geometry ) = MonitorGeometry::from_cursor() else { return; };
		let remembered = WindowSizeStore::load();
		let mut dpi_x = 96_u32;
		let mut dpi_y = 96_u32;
		unsafe { let _ = GetDpiForMonitor( geometry.monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y ); }
		let width = scale( remembered.width, dpi_x as i32 ).clamp( scale( 620, dpi_x as i32 ), ( geometry.work_width() - 48 ).max( scale( 620, dpi_x as i32 ) ) );
		let height = scale( remembered.height, dpi_y as i32 ).clamp( scale( 460, dpi_y as i32 ), ( geometry.work_height() - 48 ).max( scale( 460, dpi_y as i32 ) ) );
		let x = geometry.work_rect.left + ( geometry.work_width() - width ) / 2;
		let y = geometry.work_rect.top + ( geometry.work_height() - height ) / 2;
		self.refresh_theme();
		self.scroll_y = 0;
		unsafe {
			let _ = SetWindowPos( self.hwnd, None, x, y, width, height, Default::default() );
			let _ = ShowWindow( self.hwnd, SW_SHOW );
			let _ = SetForegroundWindow( self.hwnd );
		}
	}
	fn refresh_theme( &mut self ) {
		self.theme = SettingsTheme::system();
		let dark = self.theme.dark as i32;
		unsafe {
			let _ = DwmSetWindowAttribute( self.hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, std::ptr::from_ref( &dark ).cast(), size_of::< i32 >() as u32 );
			let _ = InvalidateRect( Some( self.hwnd ), None, false );
		}
	}
	fn update_slider( &mut self, field: SettingId, x: i32 ) {
		let Some( row ) = self.layout().row( field ).copied() else { return; };
		let dpi = unsafe { GetDpiForWindow( self.hwnd ) }.max( 96 ) as i32;
		let ratio = slider_ratio_from_x( row.control, field, dpi, x );
		match field {
			SettingId::Overlay => self.draft_preferences.start.overlay_opacity_percent = ( ratio * 100.0 ).round() as u8,
			SettingId::Blur => self.draft_preferences.start.blur_percent = ( ratio * 100.0 ).round() as u8,
			SettingId::AnimationDuration => self.draft_preferences.start.opening_duration_ms = ( ( ratio * 100.0 ).round() as u32 * 50 ).min( 5000 ),
			_ => return,
		}
		unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); }
	}
	fn undo( &mut self ) {
		if !self.is_dirty() { return; }
		self.draft_preferences = self.saved_preferences;
		unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); }
	}
	fn save( &mut self ) {
		if !self.is_dirty() { return; }
		self.draft_preferences.start.normalize();
		if let Err( error ) = self.store.save( &self.draft_preferences ) {
			show_error_dialog( "保存设置失败", &error );
			return;
		}
		match self.store.load() {
			Ok( preferences ) => {
				self.saved_preferences = preferences;
				self.draft_preferences = preferences;
				( self.on_change )( preferences.start );
			}
			Err( error ) => {
				show_error_dialog( "重新读取设置失败", &error );
				return;
			}
		}
		unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); }
	}
	fn is_dirty( &self ) -> bool {
		self.draft_preferences != self.saved_preferences
	}
	fn hit_test_slider( &self, x: i32, y: i32 ) -> Option< SettingId > {
		let layout = self.layout();
		let id = layout.hit_control( x, y )?;
		layout.row( id ).filter( |row| row.kind == ControlKind::Slider ).map( |row| row.id )
	}
	fn hit_test_choice( &self, x: i32, y: i32 ) -> Option< SettingId > {
		let layout = self.layout();
		let id = layout.hit_control( x, y )?;
		layout.row( id ).filter( |row| row.kind == ControlKind::Choice ).map( |row| row.id )
	}
	fn choose( &mut self, field: SettingId, point: POINT ) {
		let Some( selected ) = choose_choice_value( self.hwnd, field, point ) else { return; };
		match field {
			SettingId::BarColumns => self.draft_preferences.start.tile_bar_columns = selected,
			SettingId::TilesPerRow => self.draft_preferences.start.tiles_per_row = selected,
			_ => return,
		}
		self.draft_preferences.start.normalize();
		unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); }
	}
	fn layout( &self ) -> SettingsLayout {
		let mut client = RECT::default();
		unsafe { let _ = GetClientRect( self.hwnd, &mut client ); }
		let dpi = unsafe { GetDpiForWindow( self.hwnd ) }.max( 96 ) as i32;
		SettingsLayout::calculate( client, dpi, self.scroll_y )
	}
	fn scroll_to( &mut self, position: i32 ) {
		let maximum = self.layout().scroll_max;
		let position = position.clamp( 0, maximum );
		if position == self.scroll_y { return; }
		self.scroll_y = position;
		unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); }
	}
	fn begin_scroll_drag( &mut self, x: i32, y: i32 ) -> bool {
		let layout = self.layout();
		if layout.hit_scroll_thumb( x, y ) {
			let thumb_top = layout.scrollbar.unwrap().thumb.top;
			self.pointer_drag = Some( PointerDrag::Scrollbar( y - thumb_top ) );
			unsafe { SetCapture( self.hwnd ); }
			return true;
		}
		if layout.hit_scroll_track( x, y ) {
			let scrollbar = layout.scrollbar.unwrap();
			let offset = ( scrollbar.thumb.bottom - scrollbar.thumb.top ) / 2;
			self.scroll_to( layout.scroll_from_thumb( y - offset ) );
			self.pointer_drag = Some( PointerDrag::Scrollbar( offset ) );
			unsafe { SetCapture( self.hwnd ); }
			return true;
		}
		false
	}
	fn update_scroll_drag( &mut self, y: i32, offset: i32 ) {
		let layout = self.layout();
		self.scroll_to( layout.scroll_from_thumb( y - offset ) );
	}
	fn save_window_size( &self ) {
		let mut rect = RECT::default();
		if unsafe { GetWindowRect( self.hwnd, &mut rect ) }.is_ok() {
			WindowSizeStore::save( WindowSize { width: rect.right - rect.left, height: rect.bottom - rect.top } );
		}
	}
	fn paint( &self, hdc: HDC, client: RECT ) {
		let dpi = unsafe { GetDpiForWindow( self.hwnd ) }.max( 96 ) as i32;
		let layout = SettingsLayout::calculate( client, dpi, self.scroll_y );
		let painter = Painter::new( hdc, dpi );
		painter.fill( client, self.theme.background );
		painter.fill( layout.sidebar, self.theme.sidebar );
		if layout.expanded_sidebar {
			painter.text( &self.text.settings, layout.settings_title, 25, FW_SEMIBOLD.0 as i32, self.theme.text );
		}
		draw_sidebar_item( &painter, &self.theme, &layout, &self.text.start );
		painter.text( &self.text.start, layout.page_title, 30, FW_SEMIBOLD.0 as i32, self.theme.text );
		let saved = unsafe { SaveDC( hdc ) };
		unsafe { let _ = IntersectClipRect( hdc, 0, layout.viewport_top, client.right, layout.viewport_bottom ); }
		painter.text( &self.text.menu_background, layout.menu_section_title, 17, FW_SEMIBOLD.0 as i32, self.theme.text );
		painter.text( &self.text.tiles, layout.tile_section_title, 17, FW_SEMIBOLD.0 as i32, self.theme.text );
		let active_slider = match self.pointer_drag { Some( PointerDrag::Slider( field ) ) => Some( field ), _ => None };
		for row in &layout.rows {
			draw_setting_row( &painter, &self.theme, row, self.setting_view( row.id ), active_slider );
		}
		unsafe { let _ = RestoreDC( hdc, saved ); }
		let dirty = self.is_dirty();
		draw_action_button( &painter, &self.theme, layout.undo_button, &self.text.undo, false, dirty );
		draw_action_button( &painter, &self.theme, layout.save_button, &self.text.save, true, dirty );
		if let Some( scrollbar ) = layout.scrollbar {
			painter.round_rect( scrollbar.thumb, 4, self.theme.secondary_text );
		}
	}
	fn paint_buffered( &self, hdc: HDC, client: RECT ) {
		draw_buffered( hdc, client, |buffer_hdc| self.paint( buffer_hdc, client ) );
	}
	fn setting_view( &self, id: SettingId ) -> SettingView< '_ > {
		match id {
			SettingId::Overlay => SettingView { icon: "◐", text: &self.text.overlay_opacity, value: format!( "{}%", self.draft_preferences.start.overlay_opacity_percent ), minimum: "0%", maximum: "100%", ratio: self.draft_preferences.start.overlay_opacity_percent as f32 / 100.0 },
			SettingId::Blur => SettingView { icon: "≋", text: &self.text.background_blur, value: format!( "{}%", self.draft_preferences.start.blur_percent ), minimum: "0%", maximum: "100%", ratio: self.draft_preferences.start.blur_percent as f32 / 100.0 },
			SettingId::AnimationDuration => SettingView { icon: "↔", text: &self.text.animation_duration, value: format!( "{} ms", self.draft_preferences.start.opening_duration_ms ), minimum: "0 ms", maximum: "5000 ms", ratio: self.draft_preferences.start.opening_duration_ms as f32 / 5000.0 },
			SettingId::BarColumns => SettingView { icon: "▦", text: &self.text.group_columns, value: self.draft_preferences.start.tile_bar_columns.to_string(), minimum: "", maximum: "", ratio: 0.0 },
			SettingId::TilesPerRow => SettingView { icon: "≡", text: &self.text.tiles_per_row, value: self.draft_preferences.start.tiles_per_row.to_string(), minimum: "", maximum: "", ratio: 0.0 },
		}
	}
}

unsafe fn create_window( state: *mut SettingsState, large_icon: windows::Win32::UI::WindowsAndMessaging::HICON, small_icon: windows::Win32::UI::WindowsAndMessaging::HICON ) -> WindowsResult< HWND > {
	let module = unsafe { GetModuleHandleW( None )? };
	let instance = HINSTANCE( module.0 );
	let class = WNDCLASSW { lpfnWndProc: Some( settings_window_proc ), hInstance: instance, hIcon: large_icon, hCursor: unsafe { LoadCursorW( None, IDC_ARROW )? }, lpszClassName: w!( "EpStartSettingsWindow" ), ..Default::default() };
	if unsafe { RegisterClassW( &class ) } == 0 { return Err( windows::core::Error::from_thread() ); }
	let hwnd = unsafe { CreateWindowExW( Default::default(), w!( "EpStartSettingsWindow" ), w!( "ep_start" ), WS_OVERLAPPEDWINDOW, 0, 0, 1100, 720, None, None, Some( instance ), Some( state.cast::< c_void >() ) )? };
	unsafe { SendMessageW( hwnd, WM_SETICON, Some( WPARAM( ICON_SMALL as usize ) ), Some( LPARAM( small_icon.0 as isize ) ) ); }
	Ok( hwnd )
}

unsafe extern "system" fn settings_window_proc( hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if message == WM_NCCREATE {
		let creation = unsafe { &*( lparam.0 as *const CREATESTRUCTW ) };
		let state = creation.lpCreateParams.cast::< SettingsState >();
		unsafe {
			( *state ).hwnd = hwnd;
			SetWindowLongPtrW( hwnd, GWLP_USERDATA, state as isize );
		}
	}
	let state = unsafe { GetWindowLongPtrW( hwnd, GWLP_USERDATA ) as *mut SettingsState };
	if !state.is_null() {
		match message {
			WM_SHOW_SETTINGS => {
				unsafe { ( *state ).show(); }
				return LRESULT( 0 );
			}
			WM_CLOSE => {
				unsafe { let _ = ShowWindow( hwnd, SW_HIDE ); }
				trim_working_set();
				return LRESULT( 0 );
			}
			WM_LBUTTONDOWN => {
				let x = lparam.0 as i16 as i32;
				let y = ( lparam.0 >> 16 ) as i16 as i32;
				if unsafe { ( *state ).begin_scroll_drag( x, y ) } { return LRESULT( 0 ); }
				if let Some( field ) = unsafe { ( *state ).hit_test_slider( x, y ) } {
					unsafe {
						( *state ).pointer_drag = Some( PointerDrag::Slider( field ) );
						( *state ).update_slider( field, x );
						SetCapture( hwnd );
					}
				}
				return LRESULT( 0 );
			}
			WM_MOUSEMOVE => {
				let x = lparam.0 as i16 as i32;
				let y = ( lparam.0 >> 16 ) as i16 as i32;
				match unsafe { ( *state ).pointer_drag } {
					Some( PointerDrag::Slider( field ) ) => unsafe { ( *state ).update_slider( field, x ); },
					Some( PointerDrag::Scrollbar( offset ) ) => unsafe { ( *state ).update_scroll_drag( y, offset ); },
					None => {}
				}
				return LRESULT( 0 );
			}
			WM_LBUTTONUP => {
				if unsafe { ( *state ).pointer_drag.take().is_some() } {
					unsafe { let _ = ReleaseCapture(); }
				} else {
					let x = lparam.0 as i16 as i32;
					let y = ( lparam.0 >> 16 ) as i16 as i32;
					let layout = unsafe { ( *state ).layout() };
					if layout.hit_undo( x, y ) {
						unsafe { ( *state ).undo(); }
					} else if layout.hit_save( x, y ) {
						unsafe { ( *state ).save(); }
					} else if let Some( field ) = unsafe { ( *state ).hit_test_choice( x, y ) } {
						let mut point = POINT { x, y };
						unsafe {
							let _ = ClientToScreen( hwnd, &mut point );
							( *state ).choose( field, point );
						}
					}
				}
				return LRESULT( 0 );
			}
			WM_PAINT => {
				let mut paint = PAINTSTRUCT::default();
				let mut client = RECT::default();
				unsafe {
					BeginPaint( hwnd, &mut paint );
					let _ = GetClientRect( hwnd, &mut client );
					( *state ).paint_buffered( paint.hdc, client );
					let _ = EndPaint( hwnd, &paint );
				}
				return LRESULT( 0 );
			}
			WM_SIZE => {
				unsafe {
					let maximum = ( *state ).layout().scroll_max;
					( *state ).scroll_y = ( *state ).scroll_y.clamp( 0, maximum );
					let _ = InvalidateRect( Some( hwnd ), None, false );
				}
				return LRESULT( 0 );
			}
			WM_EXITSIZEMOVE => {
				unsafe { ( *state ).save_window_size(); }
				return LRESULT( 0 );
			}
			WM_MOUSEWHEEL => {
				let delta = ( wparam.0 >> 16 ) as i16 as i32;
				unsafe {
					let position = ( *state ).scroll_y - delta / 120 * 72;
					( *state ).scroll_to( position );
				}
				return LRESULT( 0 );
			}
			WM_GETMINMAXINFO => {
				let dpi = unsafe { GetDpiForWindow( hwnd ) }.max( 96 ) as i32;
				let info = unsafe { &mut *( lparam.0 as *mut MINMAXINFO ) };
				info.ptMinTrackSize.x = scale( 620, dpi );
				info.ptMinTrackSize.y = scale( 460, dpi );
				return LRESULT( 0 );
			}
			WM_SETTINGCHANGE => {
				unsafe { ( *state ).refresh_theme(); }
				return LRESULT( 0 );
			}
			WM_DPICHANGED => {
				let suggested = unsafe { &*( lparam.0 as *const RECT ) };
				unsafe {
					let _ = SetWindowPos( hwnd, None, suggested.left, suggested.top, suggested.right - suggested.left, suggested.bottom - suggested.top, SWP_NOACTIVATE );
					let _ = InvalidateRect( Some( hwnd ), None, false );
				}
				return LRESULT( 0 );
			}
			WM_ERASEBKGND => { return LRESULT( 1 ); }
			WM_DESTROY => { return LRESULT( 0 ); }
			WM_NCDESTROY => { unsafe { SetWindowLongPtrW( hwnd, GWLP_USERDATA, 0 ); } }
			_ => {}
		}
	}
	unsafe { DefWindowProcW( hwnd, message, wparam, lparam ) }
}