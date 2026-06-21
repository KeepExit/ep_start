//! ::  Project Path  ->  ep_start :: slider.rs :: slider
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 06:47 周日


use crate::ui::geometry::{ UiRect, scale };
use crate::ui::painter::Painter;
use crate::ui::theme::SettingsTheme;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::FW_NORMAL;


const LABEL_AREA_WIDTH: i32 = 64;
const LABEL_TRACK_GAP: i32 = 10;


pub( crate ) fn draw_slider_control( painter: &Painter, theme: &SettingsTheme, control: RECT, value: &str, minimum: &str, maximum: &str, ratio: f32, show_popup: bool ) {
	let control = UiRect::from( control );
	let ( track_left, track_right ) = slider_track_bounds( control.to_rect(), painter.dpi() );
	let track_y = control.center_y() + painter.scale( 4 );
	painter.text( minimum, UiRect::new( control.left, control.top, track_left - painter.scale( LABEL_TRACK_GAP ), control.bottom ), 12, FW_NORMAL.0 as i32, theme.secondary_text );
	painter.right_text( maximum, UiRect::new( track_right + painter.scale( LABEL_TRACK_GAP ), control.top, control.right, control.bottom ), 12, FW_NORMAL.0 as i32, theme.secondary_text );
	painter.round_rect( UiRect::new( track_left, track_y - painter.scale( 2 ), track_right, track_y + painter.scale( 2 ) ), 4, theme.track );
	let thumb_x = track_left + ( ( track_right - track_left ) as f32 * ratio.clamp( 0.0, 1.0 ) ).round() as i32;
	painter.round_rect( UiRect::new( track_left, track_y - painter.scale( 2 ), thumb_x, track_y + painter.scale( 2 ) ), 4, theme.accent );
	painter.antialiased_thumb( thumb_x, track_y, 9, 5, theme.thumb_outer, theme.accent );
	if show_popup {
		draw_slider_popup( painter, theme, value, track_left, track_right, thumb_x, track_y );
	}
}

pub( crate ) fn slider_ratio_from_x( control: RECT, dpi: i32, x: i32 ) -> f32 {
	let ( left, right ) = slider_track_bounds( control, dpi );
	( x - left ).clamp( 0, right - left ) as f32 / ( right - left ).max( 1 ) as f32
}

fn draw_slider_popup( painter: &Painter, theme: &SettingsTheme, value: &str, track_left: i32, track_right: i32, thumb_x: i32, track_y: i32 ) {
	let popup_width = painter.scale( ( value.chars().count() as i32 * 8 + 22 ).max( 50 ) );
	let popup_height = painter.scale( 30 );
	let popup_left = ( thumb_x - popup_width / 2 ).clamp( track_left, ( track_right - popup_width ).max( track_left ) );
	let popup_bottom = track_y - painter.scale( 11 );
	let popup = UiRect::new( popup_left, popup_bottom - popup_height, popup_left + popup_width, popup_bottom );
	painter.round_rect( popup, 6, theme.card_border );
	painter.round_rect( popup.inset( 1, 1 ), 6, theme.value_popup );
	painter.center_text( value, popup, 13, FW_NORMAL.0 as i32, theme.text );
}

fn slider_track_bounds( control: RECT, dpi: i32 ) -> ( i32, i32 ) {
	let label_width = scale( LABEL_AREA_WIDTH, dpi );
	( control.left + label_width, control.right - label_width )
}
