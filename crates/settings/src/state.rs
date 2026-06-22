use crate::ui::components::{ InteractionAnimations, InteractionId, choose_choice_value, slider_ratio_from_x };
use crate::paint::{SettingsPageView, paint_settings_page_buffered };
use crate::ui::geometry::scale;
use crate::ui::settings::{ SettingId, SettingsUi, SettingsUiLayout };
use crate::ui::theme::SettingsTheme;
use crate::host::{client_rect, dpi_for_monitor, dpi_for_window, foreground_window, request_repaint, set_dark_frame, set_window_bounds, show_window, window_rect };
use crate::size::{WindowSize, WindowSizeStore };
use configuration::{ AppPreferences, ConfigurationStore, StartPreferences, StartShortcut };
use localization::TextResources;
use platform::{ MonitorGeometry, launch_shell_restart_helper, show_error_dialog };
use windows::Win32::Foundation::{ HWND, RECT };
use windows::Win32::Graphics::Gdi::HDC;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;


pub( crate ) struct SettingsState {
	pub( crate ) hwnd: HWND,
	pub( crate ) store: ConfigurationStore,
	pub( crate ) saved_preferences: AppPreferences,
	pub( crate ) draft_preferences: AppPreferences,
	pub( crate ) text: TextResources,
	pub( crate ) theme: SettingsTheme,
	pub( crate ) ui: SettingsUi,
	pub( crate ) interactions: InteractionAnimations,
	pub( crate ) pointer_drag: Option< PointerDrag >,
	pub( crate ) scroll_y: i32,
	pub( crate ) on_change: Box< dyn FnMut( StartPreferences ) >,
}

#[derive( Clone, Copy )]
pub( crate ) enum PointerUpAction {
	None,
	ReleaseCapture,
	Choice( SettingId ),
}

#[derive( Clone, Copy )]
pub( crate ) enum PointerDrag {
	Slider( SettingId ),
	Scrollbar( i32 ),
}

impl SettingsState {
	pub( crate ) fn new( store: ConfigurationStore, preferences: AppPreferences, text: TextResources, on_change: impl FnMut( StartPreferences ) + 'static ) -> Self {
		let interactions = InteractionAnimations::new( preferences.start.open_on_start_button_click );
		Self {
			hwnd: HWND::default(),
			store,
			saved_preferences: preferences,
			draft_preferences: preferences,
			text,
			theme: SettingsTheme::system(),
			ui: SettingsUi::settings_page(),
			interactions,
			pointer_drag: None,
			scroll_y: 0,
			on_change: Box::new( on_change ),
		}
	}
	pub( crate ) fn set_hwnd( &mut self, hwnd: HWND ) {
		self.hwnd = hwnd;
	}
	pub( crate ) fn show( &mut self ) {
		let Ok( geometry ) = MonitorGeometry::from_cursor() else { return; };
		let remembered = WindowSizeStore::load();
		let ( dpi_x, dpi_y ) = dpi_for_monitor( geometry.monitor );
		let width = scale( remembered.width, dpi_x as i32 ).clamp( scale( 620, dpi_x as i32 ), ( geometry.work_width() - 48 ).max( scale( 620, dpi_x as i32 ) ) );
		let height = scale( remembered.height, dpi_y as i32 ).clamp( scale( 460, dpi_y as i32 ), ( geometry.work_height() - 48 ).max( scale( 460, dpi_y as i32 ) ) );
		let x = geometry.work_rect.left + ( geometry.work_width() - width ) / 2;
		let y = geometry.work_rect.top + ( geometry.work_height() - height ) / 2;
		self.refresh_theme();
		self.interactions.clear_pointer();
		self.scroll_y = 0;
		set_window_bounds( self.hwnd, x, y, width, height );
		show_window( self.hwnd );
		foreground_window( self.hwnd );
	}
	pub( crate ) fn refresh_theme( &mut self ) {
		self.theme = SettingsTheme::system();
		set_dark_frame( self.hwnd, self.theme.dark );
		request_repaint( self.hwnd );
	}
	pub( crate ) fn update_slider( &mut self, field: SettingId, x: i32 ) {
		let Some( row ) = self.layout().item( field ).copied() else { return; };
		let dpi = dpi_for_window( self.hwnd );
		let ratio = slider_ratio_from_x( row.control, dpi, x );
		match field {
			SettingId::Overlay => self.draft_preferences.start.overlay_opacity_percent = ( ratio * 100.0 ).round() as u8,
			SettingId::Blur => self.draft_preferences.start.blur_percent = ( ratio * 100.0 ).round() as u8,
			SettingId::AnimationDuration => self.draft_preferences.start.opening_duration_ms = ( ( ratio * 100.0 ).round() as u32 * 50 ).min( 5000 ),
			_ => return,
		}
		request_repaint( self.hwnd );
	}
	pub( crate ) fn undo( &mut self ) {
		if !self.is_dirty() { return; }
		self.draft_preferences = self.saved_preferences;
		self.interactions.set_toggle( InteractionId::Setting( SettingId::StartButtonClick ), self.draft_preferences.start.open_on_start_button_click );
		request_repaint( self.hwnd );
	}
	pub( crate ) fn save( &mut self ) {
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
				self.interactions.set_toggle( InteractionId::Setting( SettingId::StartButtonClick ), preferences.start.open_on_start_button_click );
				( self.on_change )( preferences.start );
			}
			Err( error ) => {
				show_error_dialog( "重新读取设置失败", &error );
				return;
			}
		}
		request_repaint( self.hwnd );
	}
	pub( crate ) fn is_dirty( &self ) -> bool {
		self.draft_preferences != self.saved_preferences
	}
	pub( crate ) fn choose( &mut self, field: SettingId ) {
		let Some( item ) = self.layout().item( field ).copied() else { return; };
		let current = match field {
			SettingId::Shortcut => match self.draft_preferences.start.shortcut { StartShortcut::WinShift => 0, StartShortcut::Win => 1 },
			SettingId::BarColumns => self.draft_preferences.start.tile_bar_columns,
			SettingId::TilesPerRow => self.draft_preferences.start.tiles_per_row,
			_ => return,
		};
		let Some( selected ) = choose_choice_value( self.hwnd, field, current, item.control, dpi_for_window( self.hwnd ), self.theme, &self.text ) else { return; };
		match field {
			SettingId::Shortcut => self.draft_preferences.start.shortcut = if selected == 1 { StartShortcut::Win } else { StartShortcut::WinShift },
			SettingId::BarColumns => self.draft_preferences.start.tile_bar_columns = selected,
			SettingId::TilesPerRow => self.draft_preferences.start.tiles_per_row = selected,
			_ => return,
		}
		self.draft_preferences.start.normalize();
		request_repaint( self.hwnd );
	}
	pub( crate ) fn toggle_switch( &mut self, field: SettingId ) {
		match field {
			SettingId::StartButtonClick => self.draft_preferences.start.open_on_start_button_click = !self.draft_preferences.start.open_on_start_button_click,
			_ => return,
		}
		self.interactions.set_toggle( InteractionId::Setting( field ), self.draft_preferences.start.open_on_start_button_click );
		request_repaint( self.hwnd );
	}
	pub( crate ) fn advance_interactions( &mut self ) {
		self.interactions.advance();
		request_repaint( self.hwnd );
	}
	pub( crate ) fn interactions_animating( &self ) -> bool {
		self.interactions.is_animating()
	}
	pub( crate ) fn restart_shell( &mut self ) {
		if let Err( error ) = launch_shell_restart_helper() {
			show_error_dialog( "重启失败", &error );
			return;
		}
		unsafe { PostQuitMessage( 0 ); }
	}
	pub( crate ) fn layout( &self ) -> SettingsUiLayout {
		let client = client_rect( self.hwnd );
		let dpi = dpi_for_window( self.hwnd );
		self.ui.layout( client, dpi, self.scroll_y )
	}
	pub( crate ) fn save_window_size( &self ) {
		if let Some( rect ) = window_rect( self.hwnd ) {
			WindowSizeStore::save( WindowSize { width: rect.right - rect.left, height: rect.bottom - rect.top } );
		}
	}
	pub( crate ) fn paint_buffered( &self, hdc: HDC, client: RECT ) {
		let dpi = dpi_for_window( self.hwnd );
		paint_settings_page_buffered( hdc, client, dpi, SettingsPageView {
			ui: &self.ui,
			text: &self.text,
			theme: &self.theme,
			preferences: &self.draft_preferences,
			dirty: self.is_dirty(),
			active_slider: self.active_slider(),
			interactions: &self.interactions,
			scroll_y: self.scroll_y,
		} );
	}
	fn active_slider( &self ) -> Option< SettingId > {
		match self.pointer_drag {
			Some( PointerDrag::Slider( field ) ) => Some( field ),
			_ => None,
		}
	}
}
