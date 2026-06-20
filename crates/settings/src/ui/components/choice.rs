//! ::  Project Path  ->  ep_start :: choice.rs :: choice
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 06:47 周日


use crate::ui::layout::SettingId;
use crate::ui::geometry::UiRect;
use crate::ui::painter::Painter;
use crate::ui::theme::SettingsTheme;
use std::ops::RangeInclusive;
use windows::Win32::Foundation::{ HWND, POINT, RECT };
use windows::Win32::Graphics::Gdi::{ FW_NORMAL, FW_SEMIBOLD };
use windows::Win32::UI::WindowsAndMessaging::{ AppendMenuW, CreatePopupMenu, DestroyMenu, MF_STRING, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx };
use windows::core::PCWSTR;


pub( crate ) fn draw_choice_control( painter: &Painter, theme: &SettingsTheme, control: RECT, value: &str ) {
	let control = UiRect::from( control );
	let width = painter.scale( 106 ).min( control.width() );
	let area = UiRect::new( control.right - width, control.top, control.right, control.bottom );
	painter.round_rect( area, 6, theme.track );
	painter.text( value, UiRect::new( area.left + painter.scale( 14 ), area.top, area.right - painter.scale( 28 ), area.bottom ), 15, FW_SEMIBOLD.0 as i32, theme.text );
	painter.text( "⌄", UiRect::new( area.right - painter.scale( 28 ), area.top - painter.scale( 2 ), area.right - painter.scale( 8 ), area.bottom ), 16, FW_NORMAL.0 as i32, theme.secondary_text );
}

pub( crate ) fn choose_choice_value( hwnd: HWND, field: SettingId, point: POINT ) -> Option< u8 > {
	let range = choice_values( field )?;
	let Ok( menu ) = ( unsafe { CreatePopupMenu() } ) else { return None; };
	for value in range {
		let label: Vec< u16 > = value.to_string().encode_utf16().chain( [ 0 ] ).collect();
		unsafe { let _ = AppendMenuW( menu, MF_STRING, value as usize, PCWSTR( label.as_ptr() ) ); }
	}
	let selected = unsafe { TrackPopupMenuEx( menu, ( TPM_RIGHTBUTTON | TPM_RETURNCMD ).0, point.x, point.y, hwnd, None ) }.0 as u8;
	unsafe { let _ = DestroyMenu( menu ); }
	if selected == 0 { None } else { Some( selected ) }
}

fn choice_values( field: SettingId ) -> Option< RangeInclusive< u8 > > {
	match field {
		SettingId::BarColumns => Some( 1..=6 ),
		SettingId::TilesPerRow => Some( 3..=5 ),
		_ => None,
	}
}