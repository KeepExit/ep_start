//! ::  Project Path  ->  ep_start :: button.rs :: button
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 06:46 周日


use crate::ui::painter::Painter;
use crate::ui::geometry::{ UiRect, scale };
use super::interaction::InteractionVisual;
use crate::ui::theme::{ SettingsTheme, blend_color, rgb };
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::FW_SEMIBOLD;


const INLINE_BUTTON_WIDTH: i32 = 116;
const INLINE_BUTTON_HEIGHT: i32 = 32;


pub( crate ) fn draw_action_button( painter: &Painter, theme: &SettingsTheme, area: RECT, text: &str, primary: bool, enabled: bool, interaction: InteractionVisual ) {
	let mut area = UiRect::from( area );
	let inset = ( painter.scale( 1 ) as f32 * interaction.press ).round() as i32;
	area = area.inset( inset, inset );
	let base = if primary { theme.accent } else if enabled { theme.card } else { theme.track };
	let hover = if primary { blend_color( theme.accent, rgb( 255, 255, 255 ), 0.24 ) } else { blend_color( theme.control_hover, theme.text, 0.08 ) };
	let pressed = if primary { blend_color( theme.accent, rgb( 0, 0, 0 ), 0.18 ) } else { theme.control_pressed };
	let background = blend_color( blend_color( base, hover, interaction.hover ), pressed, interaction.press );
	let foreground = if primary { rgb( 0, 0, 0 ) } else if enabled { theme.text } else { theme.secondary_text };
	painter.round_rect( area, 7, background );
	painter.center_text( text, area, 14, FW_SEMIBOLD.0 as i32, foreground );
}


pub( crate ) fn draw_setting_button( painter: &Painter, theme: &SettingsTheme, control: RECT, text: &str, interaction: InteractionVisual ) {
	let mut area = UiRect::from( setting_button_bounds( control, painter.dpi() ) );
	let inset = ( painter.scale( 1 ) as f32 * interaction.press ).round() as i32;
	area = area.inset( inset, inset );
	let hover = blend_color( theme.control_hover, theme.text, 0.10 );
	let background = blend_color( blend_color( theme.track, hover, interaction.hover ), theme.control_pressed, interaction.press );
	let border = blend_color( theme.card_border, theme.text, interaction.hover * 0.16 );
	painter.round_rect( area, 5, border );
	painter.round_rect( area.inset( painter.scale( 1 ), painter.scale( 1 ) ), 5, background );
	painter.center_text( text, area, 13, FW_SEMIBOLD.0 as i32, theme.text );
}


pub( crate ) fn setting_button_contains( control: RECT, dpi: i32, x: i32, y: i32 ) -> bool {
	UiRect::from( setting_button_bounds( control, dpi ) ).contains( x, y )
}


fn setting_button_bounds( control: RECT, dpi: i32 ) -> RECT {
	let control = UiRect::from( control );
	let width = scale( INLINE_BUTTON_WIDTH, dpi ).min( control.width() );
	let height = scale( INLINE_BUTTON_HEIGHT, dpi ).min( control.height() );
	let top = control.center_y() - height / 2;
	UiRect::new( control.right - width, top, control.right, top + height ).to_rect()
}
