//! ::  Project Path  ->  ep_start :: lib.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 20:46 周六


use serde::Deserialize;
use windows::Win32::Globalization::GetUserDefaultLocaleName;


const ZH_CN_SOURCE: &str = include_str!( "../../../assets/i18n/zh-cn.json" );
const EN_US_SOURCE: &str = include_str!( "../../../assets/i18n/en-us.json" );


#[derive( Clone, Debug, Deserialize )]
pub struct TextResources {
	pub settings: String,
	pub start: String,
	pub menu_background: String,
	pub tiles: String,
	pub overlay_opacity: SettingText,
	pub background_blur: SettingText,
	pub animation_duration: SettingText,
	pub group_columns: SettingText,
	pub tiles_per_row: SettingText,
	pub undo: String,
	pub save: String,
	pub open_settings: String,
	pub exit: String,
}


#[derive( Clone, Debug, Deserialize )]
pub struct SettingText {
	pub title: String,
	pub description: String,
}


impl TextResources {
	pub fn system() -> Result< Self, String > {
		let source = if system_locale().starts_with( "zh" ) { ZH_CN_SOURCE } else { EN_US_SOURCE };
		serde_json::from_str( source ).map_err( |error| format!( "解析内置语言资源失败：{}", error ) )
	}
}


fn system_locale() -> String {
	let mut name = [ 0u16; 85 ];
	let length = unsafe { GetUserDefaultLocaleName( &mut name ) };
	if length <= 1 { return "en-us".to_string(); }
	String::from_utf16_lossy( &name[ ..length as usize - 1 ] ).to_ascii_lowercase()
}
