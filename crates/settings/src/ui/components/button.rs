//! ::  Project Path  ->  ep_start :: button.rs :: button
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 06:46 周日


use crate::ui::painter::Painter;
use crate::ui::theme::{ SettingsTheme, rgb };
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::FW_SEMIBOLD;


pub( crate ) fn draw_action_button( painter: &Painter, theme: &SettingsTheme, area: RECT, text: &str, primary: bool, enabled: bool ) {
	let background = if primary { theme.accent } else if enabled { theme.card } else { theme.track };
	let foreground = if primary { rgb( 255, 255, 255 ) } else if enabled { theme.text } else { theme.secondary_text };
	painter.round_rect( area, 7, background );
	painter.center_text( text, area, 14, FW_SEMIBOLD.0 as i32, foreground );
}