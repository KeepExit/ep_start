//! ::  Project Path  ->  ep_start :: config.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 02:20 周六


use serde::{ Deserialize, Serialize };
use serde_json::ser::{ PrettyFormatter, Serializer };
use std::env;
use std::fs;
use std::io::Write;
use std::path::{ Path, PathBuf };


#[derive( Clone, Debug, Deserialize, Serialize )]
pub struct StartConfig {
	#[serde( rename = "bars", alias = "groups" )]
	pub bars: Vec< TileBar >,
}


#[derive( Clone, Debug, Deserialize, Serialize )]
pub struct TileBar {
	pub title: String,
	#[serde( default, skip_serializing_if = "Option::is_none" )]
	pub column: Option< u8 >,
	pub tiles: Vec< Tile >,
}


#[derive( Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize )]
pub struct TilePosition {
	pub column: u8,
	pub row: u16,
}


#[derive( Clone, Debug, Deserialize, Serialize )]
pub struct Tile {
	pub title: String,
	#[serde( default, skip_serializing_if = "Option::is_none" )]
	pub position: Option< TilePosition >,
	#[serde( default, skip_serializing_if = "String::is_empty" )]
	pub target: String,
	#[serde( default, skip_serializing_if = "String::is_empty" )]
	pub arguments: String,
	#[serde( default, skip_serializing_if = "String::is_empty" )]
	pub working_directory: String,
	#[serde( default = "default_tile_color" )]
	pub color: String,
	#[serde( default, skip_serializing_if = "Vec::is_empty" )]
	pub tiles: Vec< Tile >,
}


#[derive( Clone )]
pub struct ConfigStore {
	path: PathBuf,
}


impl ConfigStore {
	pub fn discover() -> Result< Self, String > {
		let path = Self::candidate_paths().into_iter().find( |path| path.is_file() ).ok_or_else( || "找不到 Start 配置文件 assets/start/tiles.json".to_string() )?;
		Ok( Self { path } )
	}


	pub fn load( &self ) -> Result< StartConfig, String > {
		let source = fs::read_to_string( &self.path ).map_err( |error| format!( "读取配置文件 {} 失败：{}", self.path.display(), error ) )?;
		let config: StartConfig = serde_json::from_str( &source ).map_err( |error| format!( "解析配置文件 {} 失败：{}", self.path.display(), error ) )?;
		config.validate()?;
		Ok( config )
	}


	pub fn save( &self, config: &StartConfig ) -> Result< (), String > {
		config.validate()?;
		let temporary = self.path.with_extension( "json.tmp" );
		let file = fs::File::create( &temporary ).map_err( |error| format!( "创建磁贴临时配置失败：{}", error ) )?;
		let formatter = PrettyFormatter::with_indent( b"\t" );
		let mut serializer = Serializer::with_formatter( file, formatter );
		config.serialize( &mut serializer ).map_err( |error| format!( "序列化磁贴配置失败：{}", error ) )?;
		let mut file = serializer.into_inner();
		file.write_all( b"\n" ).and_then( |_| file.sync_all() ).map_err( |error| format!( "写入磁贴配置失败：{}", error ) )?;
		drop( file );
		fs::copy( &temporary, &self.path ).map_err( |error| format!( "替换磁贴配置失败：{}", error ) )?;
		fs::remove_file( &temporary ).map_err( |error| format!( "清理磁贴临时配置失败：{}", error ) )?;
		Ok( () )
	}


	fn candidate_paths() -> Vec< PathBuf > {
		let mut paths = Vec::new();
		if let Ok( current_dir ) = env::current_dir() { paths.push( current_dir.join( "assets" ).join( "start" ).join( "tiles.json" ) ); }
		if let Ok( executable ) = env::current_exe() {
			if let Some( directory ) = executable.parent() {
				paths.push( directory.join( "start" ).join( "tiles.json" ) );
				paths.push( directory.join( "assets" ).join( "start" ).join( "tiles.json" ) );
			}
		}
		paths
	}
}


impl StartConfig {
	fn validate( &self ) -> Result< (), String > {
		if self.bars.is_empty() { return Err( "配置文件至少需要一个磁贴栏".to_string() ); }
		for bar in &self.bars {
			if bar.title.trim().is_empty() { return Err( "磁贴栏标题不能为空".to_string() ); }
			for tile in &bar.tiles { tile.validate( &bar.title )?; }
		}
		Ok( () )
	}
}


impl Tile {
	pub fn working_directory( &self ) -> Option< &Path > {
		if self.working_directory.is_empty() { None } else { Some( Path::new( &self.working_directory ) ) }
	}


	pub fn is_folder( &self ) -> bool {
		!self.tiles.is_empty()
	}


	fn validate( &self, owner: &str ) -> Result< (), String > {
		if self.title.trim().is_empty() { return Err( format!( "磁贴栏「{}」中存在空磁贴标题", owner ) ); }
		if self.is_folder() {
			for tile in &self.tiles { tile.validate( &self.title )?; }
		} else if self.target.trim().is_empty() {
			return Err( format!( "磁贴「{}」缺少启动目标", self.title ) );
		}
		Ok( () )
	}
}


fn default_tile_color() -> String {
	"#0067C0".to_string()
}


#[cfg( test )]
mod tests {
	use super::*;


	#[test]
	fn legacy_layout_without_grid_positions_remains_valid() {
		let config: StartConfig = serde_json::from_str( r##"{"bars":[{"title":"Legacy","tiles":[{"title":"App","target":"app.exe","color":"#0067C0"}]}]}"## ).unwrap();
		assert_eq!( config.bars[ 0 ].column, None );
		assert_eq!( config.bars[ 0 ].tiles[ 0 ].position, None );
		assert!( config.validate().is_ok() );
	}
}
