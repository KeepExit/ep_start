//! ::  Project Path  ->  ep_start :: paint :: settings_paint
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 14:15 周日


use crate::ui::components::{ InteractionAnimations, InteractionId, SettingView, draw_action_button, draw_setting_row, draw_sidebar_item };
use crate::ui::paint_buffer::paint_buffered;
use crate::ui::painter::Painter;
use crate::ui::settings::{ ActionId, SectionId, SettingId, SettingsUi, SidebarItemId };
use crate::ui::theme::SettingsTheme;
use configuration::{ AppPreferences, StartShortcut };
use localization::TextResources;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::{ FW_SEMIBOLD, HDC, IntersectClipRect, RestoreDC, SaveDC };


#[derive( Clone, Copy )]
pub( crate ) struct SettingsPageView< 'a > {
	pub( crate ) ui: &'a SettingsUi,
	pub( crate ) text: &'a TextResources,
	pub( crate ) theme: &'a SettingsTheme,
	pub( crate ) preferences: &'a AppPreferences,
	pub( crate ) dirty: bool,
	pub( crate ) active_slider: Option< SettingId >,
	pub( crate ) interactions: &'a InteractionAnimations,
	pub( crate ) scroll_y: i32,
}

pub( crate ) fn paint_settings_page_buffered( hdc: HDC, client: RECT, dpi: i32, view: SettingsPageView< '_ > ) {
	paint_buffered( hdc, client, |buffer_hdc| paint_settings_page( buffer_hdc, client, dpi, view ) );
}

fn paint_settings_page( hdc: HDC, client: RECT, dpi: i32, view: SettingsPageView< '_ > ) {
	let layout = view.ui.layout( client, dpi, view.scroll_y );
	let painter = Painter::new( hdc, dpi );
	painter.fill( client, view.theme.background );
	painter.fill( layout.sidebar.bounds, view.theme.sidebar );
	painter.fill( layout.content.bounds, view.theme.background );
	if layout.sidebar.expanded {
		painter.text( &view.text.settings, layout.sidebar.title, 25, FW_SEMIBOLD.0 as i32, view.theme.text );
	}
	for item in layout.sidebar.main.iter().chain( layout.sidebar.bottom.iter() ) {
		draw_sidebar_item( &painter, view.theme, item, sidebar_label( view.text, item.id ), layout.sidebar.expanded );
	}
	painter.text( &view.text.start, layout.content.page_title, 30, FW_SEMIBOLD.0 as i32, view.theme.text );
	let saved = unsafe { SaveDC( hdc ) };
	let viewport = layout.content.viewport;
	unsafe { let _ = IntersectClipRect( hdc, viewport.left, viewport.top, viewport.right, viewport.bottom ); }
	for section in &layout.content.sections {
		painter.text( section_label( view.text, section.id ), section.title, 17, FW_SEMIBOLD.0 as i32, view.theme.text );
		for item in &section.items {
			draw_setting_row( &painter, view.theme, item, setting_view( view.text, view.preferences, item.id ), view.active_slider, view.interactions.visual( InteractionId::Setting( item.id ) ) );
		}
	}
	for section in &layout.content.sub_sections {
		for item in &section.items {
			draw_setting_row( &painter, view.theme, item, setting_view( view.text, view.preferences, item.id ), view.active_slider, view.interactions.visual( InteractionId::Setting( item.id ) ) );
		}
	}
	unsafe { let _ = RestoreDC( hdc, saved ); }
	painter.fill( layout.footer.bounds, view.theme.background );
	for action in &layout.footer.actions {
		draw_action_button( &painter, view.theme, action.area, action_label( view.text, action.id ), action.primary, view.dirty, view.interactions.visual( InteractionId::Action( action.id ) ) );
	}
	if let Some( scrollbar ) = layout.content.scrollbar {
		painter.round_rect( scrollbar.thumb, 4, view.theme.secondary_text );
	}
}

fn sidebar_label( text: &TextResources, id: SidebarItemId ) -> &str {
	match id {
		SidebarItemId::Start => &text.start,
	}
}

fn section_label( text: &TextResources, id: SectionId ) -> &str {
	match id {
		SectionId::Behavior => &text.behavior,
		SectionId::MenuBackground => &text.menu_background,
		SectionId::Tiles => &text.tiles,
		SectionId::Debug => &text.debug,
	}
}

fn action_label( text: &TextResources, id: ActionId ) -> &str {
	match id {
		ActionId::Undo => &text.undo,
		ActionId::Save => &text.save,
	}
}

fn setting_view< 'a >( text: &'a TextResources, preferences: &AppPreferences, id: SettingId ) -> SettingView< 'a > {
	match id {
		SettingId::Overlay => SettingView { icon: "◐", text: &text.overlay_opacity, value: format!( "{}%", preferences.start.overlay_opacity_percent ), minimum: "0%", maximum: "100%", ratio: preferences.start.overlay_opacity_percent as f32 / 100.0 },
		SettingId::Blur => SettingView { icon: "≋", text: &text.background_blur, value: format!( "{}%", preferences.start.blur_percent ), minimum: "0%", maximum: "100%", ratio: preferences.start.blur_percent as f32 / 100.0 },
		SettingId::AnimationDuration => SettingView { icon: "↔", text: &text.animation_duration, value: format!( "{} ms", preferences.start.opening_duration_ms ), minimum: "0 ms", maximum: "5000 ms", ratio: preferences.start.opening_duration_ms as f32 / 5000.0 },
		SettingId::Shortcut => SettingView { icon: "⌨", text: &text.start_shortcut, value: match preferences.start.shortcut { StartShortcut::WinShift => text.shortcut_win_shift.clone(), StartShortcut::Win => text.shortcut_win.clone() }, minimum: "", maximum: "", ratio: 0.0 },
		SettingId::StartButtonClick => SettingView { icon: "⊞", text: &text.start_button_click, value: if preferences.start.open_on_start_button_click { text.on.clone() } else { text.off.clone() }, minimum: "", maximum: "", ratio: 0.0 },
		SettingId::BarColumns => SettingView { icon: "▦", text: &text.group_columns, value: preferences.start.tile_bar_columns.to_string(), minimum: "", maximum: "", ratio: 0.0 },
		SettingId::TilesPerRow => SettingView { icon: "≡", text: &text.tiles_per_row, value: preferences.start.tiles_per_row.to_string(), minimum: "", maximum: "", ratio: 0.0 },
		SettingId::RoundedTiles => SettingView { icon: "▢", text: &text.rounded_tiles, value: if preferences.start.rounded_tiles { text.on.clone() } else { text.off.clone() }, minimum: "", maximum: "", ratio: 0.0 },
		SettingId::RoundedTileBars => SettingView { icon: "▤", text: &text.rounded_tile_bars, value: if preferences.start.rounded_tile_bars { text.on.clone() } else { text.off.clone() }, minimum: "", maximum: "", ratio: 0.0 },
		SettingId::TileAnimationDuration => SettingView { icon: "↝", text: &text.tile_animation_duration, value: format!( "{} ms", preferences.start.tile_animation_duration_ms ), minimum: "0 ms", maximum: "1000 ms", ratio: preferences.start.tile_animation_duration_ms as f32 / 1000.0 },
		SettingId::TileBackgroundOpacity => SettingView { icon: "◩", text: &text.tile_background_opacity, value: format!( "{}%", preferences.start.tile_background_opacity_percent ), minimum: "0%", maximum: "100%", ratio: preferences.start.tile_background_opacity_percent as f32 / 100.0 },
		SettingId::TileBarBackgroundOpacity => SettingView { icon: "▥", text: &text.tile_bar_background_opacity, value: format!( "{}%", preferences.start.tile_bar_background_opacity_percent ), minimum: "0%", maximum: "100%", ratio: preferences.start.tile_bar_background_opacity_percent as f32 / 100.0 },
		SettingId::RestartShell => SettingView { icon: "↻", text: &text.restart_shell, value: text.restart_now.clone(), minimum: "", maximum: "", ratio: 0.0 },
	}
}
