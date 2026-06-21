//! ::  Project Path  ->  ep_start :: choice.rs :: choice
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 06:47 周日


use super::choice_popup;
use crate::ui::geometry::{ UiRect, scale };
use crate::ui::painter::Painter;
use crate::ui::settings::SettingId;
use crate::ui::theme::SettingsTheme;
use localization::TextResources;
use windows::Win32::Foundation::{ HWND, POINT, RECT };
use windows::Win32::Graphics::Gdi::{ ClientToScreen, FW_NORMAL };


const CONTROL_WIDTH: i32 = 116;
const CONTROL_HEIGHT: i32 = 32;


pub( crate ) fn draw_choice_control( painter: &Painter, theme: &SettingsTheme, control: RECT, value: &str ) {
	let area = UiRect::from( choice_control_bounds( control, painter.dpi() ) );
	painter.round_rect( area, 5, theme.card_border );
	painter.round_rect( area.inset( 1, 1 ), 5, theme.track );
	painter.text( value, UiRect::new( area.left + painter.scale( 12 ), area.top, area.right - painter.scale( 30 ), area.bottom ), 14, FW_NORMAL.0 as i32, theme.text );
	painter.center_text( "⌄", UiRect::new( area.right - painter.scale( 28 ), area.top - painter.scale( 2 ), area.right - painter.scale( 8 ), area.bottom ), 14, FW_NORMAL.0 as i32, theme.secondary_text );
}

pub( crate ) fn choice_control_contains( control: RECT, dpi: i32, x: i32, y: i32 ) -> bool {
	UiRect::from( choice_control_bounds( control, dpi ) ).contains( x, y )
}

pub( crate ) fn choose_choice_value( hwnd: HWND, field: SettingId, current: u8, control: RECT, dpi: i32, theme: SettingsTheme, text: &TextResources ) -> Option< u8 > {
	let options = choice_options( field, text )?;
	let mut anchor = choice_control_bounds( control, dpi );
	let mut top_left = POINT { x: anchor.left, y: anchor.top };
	let mut bottom_right = POINT { x: anchor.right, y: anchor.bottom };
	unsafe {
		let _ = ClientToScreen( hwnd, &mut top_left );
		let _ = ClientToScreen( hwnd, &mut bottom_right );
	}
	anchor = RECT { left: top_left.x, top: top_left.y, right: bottom_right.x, bottom: bottom_right.y };
	choice_popup::show_choice_popup( hwnd, anchor, &options, current, theme )
}

fn choice_options( field: SettingId, text: &TextResources ) -> Option< Vec< choice_popup::ChoiceOption > > {
	match field {
		SettingId::Shortcut => Some( vec![ choice_popup::ChoiceOption::new( 0, text.shortcut_win_shift.clone() ), choice_popup::ChoiceOption::new( 1, text.shortcut_win.clone() ) ] ),
		SettingId::BarColumns => Some( ( 1..=6 ).map( choice_popup::ChoiceOption::number ).collect() ),
		SettingId::TilesPerRow => Some( ( 3..=5 ).map( choice_popup::ChoiceOption::number ).collect() ),
		_ => None,
	}
}

fn choice_control_bounds( control: RECT, dpi: i32 ) -> RECT {
	let control = UiRect::from( control );
	let width = scale( CONTROL_WIDTH, dpi ).min( control.width() );
	let height = scale( CONTROL_HEIGHT, dpi ).min( control.height() );
	let top = control.center_y() - height / 2;
	UiRect::new( control.right - width, top, control.right, top + height ).to_rect()
}
