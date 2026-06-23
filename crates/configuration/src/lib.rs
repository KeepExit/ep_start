//! ::  Project Path  ->  ep_start :: lib.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 18:20 周六


use serde::{ Deserialize, Serialize };
use serde_json::ser::{ PrettyFormatter, Serializer };
use std::env;
use std::fs;
use std::io::Write;
use std::path::{ Path, PathBuf };


#[derive( Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize )]
#[serde( rename_all = "kebab-case" )]
pub enum StartShortcut {
	#[default]
	WinShift,
	Win,
}


#[derive( Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize )]
pub struct StartPreferences {
	pub overlay_opacity_percent: u8,
	pub blur_percent: u8,
	pub opening_duration_ms: u32,
	#[serde( default )]
	pub shortcut: StartShortcut,
	#[serde( default = "default_open_on_start_button_click" )]
	pub open_on_start_button_click: bool,
	#[serde( default = "default_rounded_tiles" )]
	pub rounded_tiles: bool,
	#[serde( default = "default_rounded_tile_bars" )]
	pub rounded_tile_bars: bool,
	#[serde( default = "default_tile_animation_duration_ms" )]
	pub tile_animation_duration_ms: u32,
	#[serde( default = "default_tile_background_opacity_percent" )]
	pub tile_background_opacity_percent: u8,
	#[serde( default = "default_tile_bar_background_opacity_percent" )]
	pub tile_bar_background_opacity_percent: u8,
	#[serde( default = "default_bar_columns", alias = "tile_group_columns" )]
	pub tile_bar_columns: u8,
	#[serde( default = "default_tiles_per_row" )]
	pub tiles_per_row: u8,
}


#[derive( Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize )]
pub struct AppPreferences {
	pub start: StartPreferences,
}


#[derive( Clone )]
pub struct ConfigurationStore {
	path: PathBuf,
}


impl ConfigurationStore {
	pub fn discover() -> Result< Self, String > {
		let path = candidate_paths().into_iter().find( |path| path.is_file() ).ok_or_else( || "找不到配置文件 assets/settings.json".to_string() )?;
		Ok( Self { path } )
	}


	pub fn load( &self ) -> Result< AppPreferences, String > {
		let source = fs::read_to_string( &self.path ).map_err( |error| format!( "读取配置文件 {} 失败：{}", self.path.display(), error ) )?;
		let mut preferences: AppPreferences = serde_json::from_str( &source ).map_err( |error| format!( "解析配置文件 {} 失败：{}", self.path.display(), error ) )?;
		preferences.start.normalize();
		Ok( preferences )
	}


	pub fn save( &self, preferences: &AppPreferences ) -> Result< (), String > {
		let temporary = self.path.with_extension( "json.tmp" );
		let file = fs::File::create( &temporary ).map_err( |error| format!( "创建临时配置文件失败：{}", error ) )?;
		let formatter = PrettyFormatter::with_indent( b"\t" );
		let mut serializer = Serializer::with_formatter( file, formatter );
		preferences.serialize( &mut serializer ).map_err( |error| format!( "序列化配置失败：{}", error ) )?;
		let mut file = serializer.into_inner();
		file.write_all( b"\n" ).and_then( |_| file.sync_all() ).map_err( |error| format!( "写入配置失败：{}", error ) )?;
		drop( file );
		fs::copy( &temporary, &self.path ).map_err( |error| format!( "替换配置文件失败：{}", error ) )?;
		fs::remove_file( &temporary ).map_err( |error| format!( "清理临时配置文件失败：{}", error ) )?;
		Ok( () )
	}


	pub fn path( &self ) -> &Path {
		&self.path
	}
}


impl StartPreferences {
	pub fn normalize( &mut self ) {
		self.overlay_opacity_percent = self.overlay_opacity_percent.min( 100 );
		self.blur_percent = self.blur_percent.min( 100 );
		self.opening_duration_ms = self.opening_duration_ms.min( 5000 ) / 50 * 50;
		self.tile_animation_duration_ms = self.tile_animation_duration_ms.min( 1000 ) / 10 * 10;
		self.tile_background_opacity_percent = self.tile_background_opacity_percent.min( 100 );
		self.tile_bar_background_opacity_percent = self.tile_bar_background_opacity_percent.min( 100 );
		self.tile_bar_columns = self.tile_bar_columns.clamp( 1, 6 );
		self.tiles_per_row = self.tiles_per_row.clamp( 3, 5 );
	}
}


const fn default_bar_columns() -> u8 {
	3
}


const fn default_open_on_start_button_click() -> bool {
	true
}


const fn default_rounded_tiles() -> bool {
	true
}


const fn default_rounded_tile_bars() -> bool {
	true
}


const fn default_tile_animation_duration_ms() -> u32 {
	220
}


const fn default_tile_background_opacity_percent() -> u8 {
	64
}


const fn default_tile_bar_background_opacity_percent() -> u8 {
	64
}


const fn default_tiles_per_row() -> u8 {
	4
}


fn candidate_paths() -> Vec< PathBuf > {
	let mut paths = Vec::new();
	if let Ok( current_dir ) = env::current_dir() { paths.push( current_dir.join( "assets" ).join( "settings.json" ) ); }
	if let Ok( executable ) = env::current_exe() {
		if let Some( directory ) = executable.parent() { paths.push( directory.join( "assets" ).join( "settings.json" ) ); }
	}
	paths
}


#[cfg( test )]
mod tests {
	use super::*;


	#[test]
	fn missing_shortcut_uses_win_shift() {
		let preferences: AppPreferences = serde_json::from_str( r#"{"start":{"overlay_opacity_percent":50,"blur_percent":0,"opening_duration_ms":250,"tile_bar_columns":3,"tiles_per_row":4}}"# ).unwrap();
		assert_eq!( preferences.start.shortcut, StartShortcut::WinShift );
		assert!( preferences.start.open_on_start_button_click );
		assert!( preferences.start.rounded_tiles );
		assert!( preferences.start.rounded_tile_bars );
		assert_eq!( preferences.start.tile_animation_duration_ms, 220 );
		assert_eq!( preferences.start.tile_background_opacity_percent, 64 );
		assert_eq!( preferences.start.tile_bar_background_opacity_percent, 64 );
	}


	#[test]
	fn shortcut_names_are_stable() {
		assert_eq!( serde_json::to_string( &StartShortcut::WinShift ).unwrap(), "\"win-shift\"" );
		assert_eq!( serde_json::to_string( &StartShortcut::Win ).unwrap(), "\"win\"" );
	}
}
