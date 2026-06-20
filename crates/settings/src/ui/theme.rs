//! ::  Project Path  ->  ep_start :: theme.rs :: theme
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 06:10 周日


use std::ffi::c_void;
use std::mem::size_of;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
use windows::Win32::System::Registry::{ HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW };
use windows::core::w;


#[derive( Clone, Copy )]
pub( crate ) struct SettingsTheme {
	pub( crate ) background: COLORREF,
	pub( crate ) sidebar: COLORREF,
	pub( crate ) card: COLORREF,
	pub( crate ) card_border: COLORREF,
	pub( crate ) text: COLORREF,
	pub( crate ) secondary_text: COLORREF,
	pub( crate ) track: COLORREF,
	pub( crate ) thumb_outer: COLORREF,
	pub( crate ) value_popup: COLORREF,
	pub( crate ) accent: COLORREF,
	pub( crate ) dark: bool,
}

impl SettingsTheme {
	pub( crate ) fn system() -> Self {
		let dark = !apps_use_light_theme();
		let accent = system_accent();
		if dark {
			Self { background: rgb( 32, 32, 32 ), sidebar: rgb( 28, 28, 28 ), card: rgb( 45, 45, 45 ), card_border: rgb( 62, 62, 62 ), text: rgb( 250, 250, 250 ), secondary_text: rgb( 190, 190, 190 ), track: rgb( 76, 76, 76 ), thumb_outer: rgb( 105, 105, 105 ), value_popup: rgb( 52, 52, 52 ), accent, dark }
		} else {
			Self { background: rgb( 243, 243, 243 ), sidebar: rgb( 238, 238, 238 ), card: rgb( 251, 251, 251 ), card_border: rgb( 210, 210, 210 ), text: rgb( 25, 25, 25 ), secondary_text: rgb( 92, 92, 92 ), track: rgb( 218, 218, 218 ), thumb_outer: rgb( 255, 255, 255 ), value_popup: rgb( 255, 255, 255 ), accent, dark }
		}
	}
}

fn apps_use_light_theme() -> bool {
	let mut value = 0u32;
	let mut size = size_of::< u32 >() as u32;
	let result = unsafe { RegGetValueW( HKEY_CURRENT_USER, w!( "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize" ), w!( "AppsUseLightTheme" ), RRF_RT_REG_DWORD, None, Some( ( &mut value as *mut u32 ).cast::< c_void >() ), Some( &mut size ) ) };
	result.0 == 0 && value != 0
}
fn system_accent() -> COLORREF {
	let mut value = 0u32;
	let mut opaque = windows::core::BOOL::default();
	if unsafe { DwmGetColorizationColor( &mut value, &mut opaque ) }.is_ok() {
		return rgb( ( ( value >> 16 ) & 0xFF ) as u8, ( ( value >> 8 ) & 0xFF ) as u8, ( value & 0xFF ) as u8 );
	}
	rgb( 0, 120, 212 )
}
pub( crate ) const fn rgb( red: u8, green: u8, blue: u8 ) -> COLORREF {
	COLORREF( red as u32 | ( green as u32 ) << 8 | ( blue as u32 ) << 16 )
}