//! ::  Project Path  ->  ep_start :: sidebar.rs :: sidebar
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 06:48 周日


use crate::ui::layout::SettingsLayout;
use crate::ui::geometry::UiRect;
use crate::ui::painter::Painter;
use crate::ui::theme::SettingsTheme;
use windows::Win32::Graphics::Gdi::{ FW_NORMAL, FW_SEMIBOLD };


pub( crate ) fn draw_sidebar_item( painter: &Painter, theme: &SettingsTheme, layout: &SettingsLayout, label: &str ) {
	let nav = UiRect::from( layout.sidebar_nav );
	painter.round_rect( nav, 7, theme.card );
	painter.fill( UiRect::new( nav.left, nav.top + painter.scale( 10 ), nav.left + painter.scale( 4 ), nav.bottom - painter.scale( 10 ) ), theme.accent );
	painter.text( "▦", UiRect::new( nav.left + painter.scale( 20 ), nav.top, nav.left + painter.scale( 56 ), nav.bottom ), 20, FW_NORMAL.0 as i32, theme.accent );
	if layout.expanded_sidebar {
		painter.text( label, UiRect::new( nav.left + painter.scale( 60 ), nav.top, nav.right - painter.scale( 10 ), nav.bottom ), 16, FW_SEMIBOLD.0 as i32, theme.text );
	}
}