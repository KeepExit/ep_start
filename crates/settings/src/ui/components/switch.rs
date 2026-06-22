//! ::  Project Path  ->  ep_start :: switch.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/22 02:46 周一


use crate::ui::geometry::{ UiRect, scale };
use crate::ui::painter::Painter;
use super::interaction::InteractionVisual;
use crate::ui::theme::{ SettingsTheme, blend_color, rgb };
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::FW_NORMAL;


const CONTROL_WIDTH: i32 = 50;
const CONTROL_HEIGHT: i32 = 24;
const LABEL_WIDTH: i32 = 36;
const LABEL_GAP: i32 = 10;


pub( crate ) fn draw_switch_control( painter: &Painter, theme: &SettingsTheme, control: RECT, label: &str, interaction: InteractionVisual ) {
	let area = UiRect::from( switch_control_bounds( control, painter.dpi() ) );
	let label_right = area.left - painter.scale( LABEL_GAP );
	painter.right_text( label, UiRect::new( label_right - painter.scale( LABEL_WIDTH ), control.top, label_right, control.bottom ), 14, FW_NORMAL.0 as i32, theme.text );
	let off = blend_color( theme.track, theme.control_hover, interaction.hover );
	let on = blend_color( theme.accent, rgb( 255, 255, 255 ), interaction.hover * 0.12 );
	let background = blend_color( off, on, interaction.toggle );
	painter.round_rect( area, CONTROL_HEIGHT, theme.card_border );
	painter.round_rect( area.inset( painter.scale( 1 ), painter.scale( 1 ) ), CONTROL_HEIGHT - 2, background );
	let radius_logical = 7 + interaction.press.round() as i32;
	let radius = painter.scale( radius_logical );
	let margin = painter.scale( 4 );
	let left = area.left + margin + radius;
	let right = area.right - margin - radius;
	let center_x = left + ( ( right - left ) as f32 * smooth_step( interaction.toggle ) ).round() as i32;
	painter.antialiased_thumb( center_x, area.center_y(), radius_logical, ( radius_logical - 1 ).max( 1 ), theme.switch_thumb, theme.switch_thumb );
}


pub( crate ) fn switch_control_contains( control: RECT, dpi: i32, x: i32, y: i32 ) -> bool {
	let area = UiRect::from( switch_control_bounds( control, dpi ) );
	UiRect::new( area.left - scale( LABEL_GAP + LABEL_WIDTH, dpi ), area.top, area.right, area.bottom ).contains( x, y )
}


fn smooth_step( value: f32 ) -> f32 {
	let value = value.clamp( 0.0, 1.0 );
	value * value * ( 3.0 - 2.0 * value )
}


fn switch_control_bounds( control: RECT, dpi: i32 ) -> RECT {
	let control = UiRect::from( control );
	let width = scale( CONTROL_WIDTH, dpi ).min( control.width() );
	let height = scale( CONTROL_HEIGHT, dpi ).min( control.height() );
	let top = control.center_y() - height / 2;
	UiRect::new( control.right - width, top, control.right, top + height ).to_rect()
}
