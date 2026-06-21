//! ::  Project Path  ->  ep_start :: sidebar.rs :: sidebar
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 06:48 周日


use crate::ui::geometry::UiRect;
use crate::ui::painter::Painter;
use crate::ui::settings::SidebarItemLayout;
use crate::ui::theme::SettingsTheme;
use windows::Win32::Graphics::Gdi::FW_NORMAL;


const CONTENT_PADDING: i32 = 12;
const ICON_SIZE: i32 = 20;
const ICON_TEXT_GAP: i32 = 10;
const ACCENT_WIDTH: i32 = 4;
const ACCENT_HEIGHT: i32 = 20;


struct SidebarItemGeometry {
	accent: UiRect,
	icon: UiRect,
	text: UiRect,
}


pub( crate ) fn draw_sidebar_item( painter: &Painter, theme: &SettingsTheme, item: &SidebarItemLayout, label: &str, expanded: bool ) {
	let area = UiRect::from( item.area );
	let geometry = SidebarItemGeometry::calculate( painter, area );
	if item.selected {
		painter.round_rect( item.area, 6, theme.card );
		painter.round_rect( geometry.accent, 3, theme.accent );
	}
	painter.text( item.icon, geometry.icon, 20, FW_NORMAL.0 as i32, theme.text );
	if expanded {
		painter.text( label, geometry.text, 14, FW_NORMAL.0 as i32, theme.text );
	}
}


impl SidebarItemGeometry {
	fn calculate( painter: &Painter, area: UiRect ) -> Self {
		let padding = painter.scale( CONTENT_PADDING );
		let icon_size = painter.scale( ICON_SIZE );
		let icon_left = area.left + padding;
		let icon_top = area.center_y() - icon_size / 2;
		let icon = UiRect::new( icon_left, icon_top, icon_left + icon_size, icon_top + icon_size );
		let accent_width = painter.scale( ACCENT_WIDTH );
		let accent_height = painter.scale( ACCENT_HEIGHT );
		let accent_top = area.center_y() - accent_height / 2;
		let accent = UiRect::new( area.left, accent_top, area.left + accent_width, accent_top + accent_height );
		let text = UiRect::new( icon.right + painter.scale( ICON_TEXT_GAP ), area.top, area.right - padding, area.bottom );
		Self { accent, icon, text }
	}
}
