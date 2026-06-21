//! ::  Project Path  ->  ep_start :: setting_row.rs :: setting_row
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 06:48 周日


use super::choice::draw_choice_control;
use super::slider::draw_slider_control;

use crate::ui::painter::Painter;
use crate::ui::settings::{ ControlKind, SettingId, SettingItemLayout };
use crate::ui::theme::SettingsTheme;
use localization::SettingText;
use windows::Win32::Graphics::Gdi::{ FW_NORMAL, FW_SEMIBOLD };


pub( crate ) struct SettingView< 'a > {
	pub icon: &'static str,
	pub text: &'a SettingText,
	pub value: String,
	pub minimum: &'static str,
	pub maximum: &'static str,
	pub ratio: f32,
}

pub( crate ) fn draw_setting_row( painter: &Painter, theme: &SettingsTheme, row: &SettingItemLayout, view: SettingView< '_ >, active_slider: Option< SettingId > ) {
	painter.round_rect( row.card, 7, theme.card );
	painter.text( view.icon, row.icon, 30, FW_NORMAL.0 as i32, theme.text );
	painter.text( &view.text.title, row.title, 16, FW_SEMIBOLD.0 as i32, theme.text );
	painter.text( &view.text.description, row.description, 13, FW_NORMAL.0 as i32, theme.secondary_text );
	match row.control_kind {
		ControlKind::Slider => draw_slider_control( painter, theme, row.control, &view.value, view.minimum, view.maximum, view.ratio, active_slider.is_some_and( |field| field == row.id ) ),
		ControlKind::Choice => draw_choice_control( painter, theme, row.control, &view.value ),
	}
}
