//! ::  Project Path  ->  ep_start :: layout.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 21:40 周六


use crate::config::{ StartConfig, TileBar, TilePosition };
use configuration::StartPreferences;
use std::collections::{ BTreeSet, HashMap };
use std::sync::Arc;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;


const CONTENT_MARGIN_X: f32 = 72.0;
const CONTENT_MARGIN_Y: f32 = 44.0;
const BAR_TITLE_HEIGHT: f32 = 34.0;
const BAR_TITLE_GAP: f32 = 8.0;
const BAR_COLUMN_GAP: f32 = 30.0;
const BAR_STACK_GAP: f32 = 28.0;
const BAR_PADDING_X: f32 = 12.0;
const BAR_PADDING_TOP: f32 = 12.0;
const BAR_PADDING_BOTTOM: f32 = 18.0;
const BAR_HIT_EXPANSION_RATIO: f32 = 0.42;
const TILE_GAP: f32 = 6.0;
const FOLDER_PADDING: f32 = 14.0;
const FOLDER_COLUMNS: usize = 3;


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub struct TileAddress {
	pub bar_index: usize,
	pub tile_index: usize,
}


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub struct FolderTileAddress {
	pub owner: TileAddress,
	pub tile_index: usize,
}


#[derive( Clone, Copy )]
pub struct BarAreaRegion {
	pub column: usize,
	pub rect: D2D_RECT_F,
	pub tile_origin_x: f32,
	pub unit_size: f32,
	pub units_per_row: usize,
}


#[derive( Clone, Copy )]
pub struct BarRegion {
	pub bar_index: usize,
	pub column: usize,
	pub rect: D2D_RECT_F,
	pub title_rect: D2D_RECT_F,
	pub tile_origin_x: f32,
	pub tile_origin_y: f32,
	pub unit_size: f32,
	pub units_per_row: usize,
}


#[derive( Clone, Copy )]
pub struct TileRegion {
	pub address: TileAddress,
	pub rect: D2D_RECT_F,
}


#[derive( Clone )]
pub struct FolderPanelRegion {
	pub owner: TileAddress,
	pub rect: D2D_RECT_F,
	pub tiles: Vec< FolderTileRegion >,
}


#[derive( Clone, Copy )]
pub struct FolderTileRegion {
	pub address: FolderTileAddress,
	pub rect: D2D_RECT_F,
}


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub enum DragSource {
	Tile( TileAddress ),
	Bar( usize ),
}


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub enum DropTarget {
	Tile { bar_index: usize, position: TilePosition },
	NewBar { column: usize, stack_index: usize, position: TilePosition },
	Bar { column: usize, stack_index: usize },
}


#[derive( Clone, Debug )]
pub struct DragVisual {
	pub source: DragSource,
	pub preview_source: DragSource,
	pub origin_rect: D2D_RECT_F,
	pub delta_x: f32,
	pub delta_y: f32,
	pub target: DropTarget,
	pub reflow_progress: f32,
	pub reflow_origins: Arc< HashMap< u64, D2D_RECT_F > >,
}


#[derive( Clone, Copy, Debug )]
pub struct TileDropVisual {
	pub runtime_id: u64,
	pub from_rect: D2D_RECT_F,
	pub to_rect: D2D_RECT_F,
	pub progress: f32,
}


#[derive( Clone, Default )]
pub struct TileLayout {
	pub areas: Vec< BarAreaRegion >,
	pub bars: Vec< BarRegion >,
	pub tiles: Vec< TileRegion >,
	pub folder_panel: Option< FolderPanelRegion >,
}


impl TileLayout {
	pub fn calculate( &mut self, client_width: f32, client_height: f32, config: &StartConfig, preferences: &StartPreferences, open_folder: Option< TileAddress > ) {
		self.areas.clear();
		self.bars.clear();
		self.tiles.clear();
		self.folder_panel = None;
		let bar_columns = preferences.tile_bar_columns.clamp( 1, 6 ) as usize;
		let tiles_per_row = preferences.tiles_per_row.clamp( 3, 5 ) as usize;
		let units_per_row = tiles_per_row * 2;
		let available_width = ( client_width - CONTENT_MARGIN_X * 2.0 - BAR_COLUMN_GAP * bar_columns.saturating_sub( 1 ) as f32 ).max( 1.0 );
		let area_width = available_width / bar_columns as f32;
		let unit_size = ( ( area_width - BAR_PADDING_X * 2.0 - TILE_GAP * units_per_row.saturating_sub( 1 ) as f32 ) / units_per_row as f32 ).max( 1.0 );
		let mut column_tops = vec![ CONTENT_MARGIN_Y; bar_columns ];
		for column in 0..bar_columns {
			let left = CONTENT_MARGIN_X + column as f32 * ( area_width + BAR_COLUMN_GAP );
			self.areas.push( BarAreaRegion { column, rect: D2D_RECT_F { left, top: CONTENT_MARGIN_Y, right: left + area_width, bottom: ( client_height - CONTENT_MARGIN_Y ).max( CONTENT_MARGIN_Y ) }, tile_origin_x: left + BAR_PADDING_X, unit_size, units_per_row } );
		}
		for ( bar_index, bar ) in config.bars.iter().enumerate() {
			let column = bar.column.map( usize::from ).unwrap_or( bar_index % bar_columns ).min( bar_columns - 1 );
			let area = self.areas[ column ];
			let positions = resolved_tile_positions( bar, tiles_per_row );
			let rows = positions.iter().zip( &bar.tiles ).map( |( position, tile )| position.row as usize + tile.size.grid_height() ).max().unwrap_or( 2 );
			let top = column_tops[ column ];
			let title_top = top + BAR_PADDING_TOP;
			let tile_origin_y = title_top + BAR_TITLE_HEIGHT + BAR_TITLE_GAP;
			let tile_height = rows as f32 * unit_size + rows.saturating_sub( 1 ) as f32 * TILE_GAP;
			let rect = D2D_RECT_F { left: area.rect.left, top, right: area.rect.right, bottom: tile_origin_y + tile_height + BAR_PADDING_BOTTOM };
			let title_rect = D2D_RECT_F { left: rect.left + BAR_PADDING_X, top: title_top, right: rect.right - BAR_PADDING_X, bottom: title_top + BAR_TITLE_HEIGHT };
			let tile_origin_x = rect.left + BAR_PADDING_X;
			self.bars.push( BarRegion { bar_index, column, rect, title_rect, tile_origin_x, tile_origin_y, unit_size, units_per_row } );
			for ( tile_index, position ) in positions.into_iter().enumerate() {
				let tile = &bar.tiles[ tile_index ];
				let tile_left = tile_origin_x + position.column as f32 * ( unit_size + TILE_GAP );
				let tile_top = tile_origin_y + position.row as f32 * ( unit_size + TILE_GAP );
				let tile_width = tile.size.grid_width() as f32 * unit_size + tile.size.grid_width().saturating_sub( 1 ) as f32 * TILE_GAP;
				let tile_height = tile.size.grid_height() as f32 * unit_size + tile.size.grid_height().saturating_sub( 1 ) as f32 * TILE_GAP;
				self.tiles.push( TileRegion { address: TileAddress { bar_index, tile_index }, rect: D2D_RECT_F { left: tile_left, top: tile_top, right: tile_left + tile_width, bottom: tile_top + tile_height } } );
			}
			column_tops[ column ] = rect.bottom + BAR_STACK_GAP;
		}
		if let Some( owner ) = open_folder { self.calculate_folder_panel( client_width, client_height, config, owner ); }
	}


	pub fn hit_test( &self, x: f32, y: f32 ) -> Option< TileAddress > {
		self.tiles.iter().find( |tile| contains( &tile.rect, x, y ) ).map( |tile| tile.address )
	}


	pub fn hit_test_folder_tile( &self, x: f32, y: f32 ) -> Option< FolderTileAddress > {
		self.folder_panel.as_ref()?.tiles.iter().find( |tile| contains( &tile.rect, x, y ) ).map( |tile| tile.address )
	}


	pub fn folder_contains( &self, x: f32, y: f32 ) -> bool {
		self.folder_panel.as_ref().is_some_and( |panel| contains( &panel.rect, x, y ) )
	}


	pub fn hit_test_bar_title( &self, x: f32, y: f32 ) -> Option< usize > {
		self.bars.iter().find( |bar| contains( &bar.title_rect, x, y ) ).map( |bar| bar.bar_index )
	}


	pub fn hit_test_bar( &self, x: f32, y: f32 ) -> Option< usize > {
		self.bars.iter().find( |bar| contains( &bar.rect, x, y ) ).map( |bar| bar.bar_index )
	}


	pub fn tile_drop_target( &self, x: f32, y: f32 ) -> Option< DropTarget > {
		let bar = self.bars.iter().filter( |bar| contains( &expanded( &bar.rect, bar.unit_size * 2.0 * BAR_HIT_EXPANSION_RATIO ), x, y ) ).min_by( |left, right| distance_squared( &left.rect, x, y ).total_cmp( &distance_squared( &right.rect, x, y ) ) );
		if let Some( bar ) = bar { return Some( DropTarget::Tile { bar_index: bar.bar_index, position: tile_position( bar, x, y, 1 ) } ); }
		let area = self.areas.iter().find( |area| contains( &area.rect, x, y ) )?;
		Some( DropTarget::NewBar { column: area.column, stack_index: self.stack_index( area.column, y ), position: area_tile_position( area, x, 1 ) } )
	}


	pub fn dragged_tile_drop_target( &self, pointer_x: f32, pointer_y: f32, grab_offset_x: f32, grab_offset_y: f32, grid_width: usize ) -> Option< DropTarget > {
		let bar = self.bars.iter().filter( |bar| contains( &expanded( &bar.rect, bar.unit_size * 2.0 * BAR_HIT_EXPANSION_RATIO ), pointer_x, pointer_y ) ).min_by( |left, right| distance_squared( &left.rect, pointer_x, pointer_y ).total_cmp( &distance_squared( &right.rect, pointer_x, pointer_y ) ) );
		if let Some( bar ) = bar { return Some( DropTarget::Tile { bar_index: bar.bar_index, position: tile_position( bar, pointer_x - grab_offset_x, pointer_y - grab_offset_y, grid_width ) } ); }
		let area = self.areas.iter().find( |area| contains( &area.rect, pointer_x, pointer_y ) )?;
		Some( DropTarget::NewBar { column: area.column, stack_index: self.stack_index( area.column, pointer_y ), position: area_tile_position( area, pointer_x - grab_offset_x, grid_width ) } )
	}


	pub fn bar_drop_target( &self, x: f32, y: f32 ) -> Option< DropTarget > {
		let area = self.areas.iter().find( |area| contains( &area.rect, x, y ) )?;
		Some( DropTarget::Bar { column: area.column, stack_index: self.stack_index( area.column, y ) } )
	}


	pub fn tile_rect( &self, address: TileAddress ) -> Option< D2D_RECT_F > {
		self.tiles.iter().find( |tile| tile.address == address ).map( |tile| tile.rect )
	}


	pub fn bar_rect( &self, bar_index: usize ) -> Option< D2D_RECT_F > {
		self.bars.iter().find( |bar| bar.bar_index == bar_index ).map( |bar| bar.rect )
	}


	pub fn drop_rect( &self, target: DropTarget ) -> Option< D2D_RECT_F > {
		match target {
			DropTarget::Tile { .. } => None,
			DropTarget::NewBar { column, stack_index, .. } | DropTarget::Bar { column, stack_index } => self.stack_drop_rect( column, stack_index ),
		}
	}


	fn stack_drop_rect( &self, column: usize, stack_index: usize ) -> Option< D2D_RECT_F > {
		let area = self.areas.iter().find( |area| area.column == column )?;
		let bars: Vec< &BarRegion > = self.bars.iter().filter( |bar| bar.column == column ).collect();
		let y = if stack_index == 0 { area.rect.top } else if stack_index >= bars.len() { bars.last().map( |bar| bar.rect.bottom + BAR_STACK_GAP * 0.5 ).unwrap_or( area.rect.top ) } else { ( bars[ stack_index - 1 ].rect.bottom + bars[ stack_index ].rect.top ) * 0.5 };
		Some( D2D_RECT_F { left: area.rect.left, top: y - 2.0, right: area.rect.right, bottom: y + 2.0 } )
	}


	fn stack_index( &self, column: usize, y: f32 ) -> usize {
		let bars: Vec< &BarRegion > = self.bars.iter().filter( |bar| bar.column == column ).collect();
		bars.iter().position( |bar| y < ( bar.rect.top + bar.rect.bottom ) * 0.5 ).unwrap_or( bars.len() )
	}


	fn calculate_folder_panel( &mut self, client_width: f32, client_height: f32, config: &StartConfig, owner: TileAddress ) {
		let Some( owner_rect ) = self.tile_rect( owner ) else { return; };
		let Some( folder ) = config.bars.get( owner.bar_index ).and_then( |bar| bar.tiles.get( owner.tile_index ) ) else { return; };
		if !folder.is_folder() { return; }
		let tile_size = ( owner_rect.right - owner_rect.left ).max( 1.0 );
		let rows = folder.tiles.len().div_ceil( FOLDER_COLUMNS ).max( 1 );
		let width = FOLDER_PADDING * 2.0 + tile_size * FOLDER_COLUMNS as f32 + TILE_GAP * FOLDER_COLUMNS.saturating_sub( 1 ) as f32;
		let height = FOLDER_PADDING * 2.0 + tile_size * rows as f32 + TILE_GAP * rows.saturating_sub( 1 ) as f32;
		let preferred_left = owner_rect.right + TILE_GAP;
		let left = if preferred_left + width <= client_width - CONTENT_MARGIN_X { preferred_left } else { ( owner_rect.left - TILE_GAP - width ).max( CONTENT_MARGIN_X ) };
		let top = owner_rect.top.clamp( CONTENT_MARGIN_Y, ( client_height - CONTENT_MARGIN_Y - height ).max( CONTENT_MARGIN_Y ) );
		let rect = D2D_RECT_F { left, top, right: ( left + width ).min( client_width - CONTENT_MARGIN_X ), bottom: ( top + height ).min( client_height - CONTENT_MARGIN_Y ) };
		let mut tiles = Vec::with_capacity( folder.tiles.len() );
		for ( tile_index, _ ) in folder.tiles.iter().enumerate() {
			let column = tile_index % FOLDER_COLUMNS;
			let row = tile_index / FOLDER_COLUMNS;
			let tile_left = rect.left + FOLDER_PADDING + column as f32 * ( tile_size + TILE_GAP );
			let tile_top = rect.top + FOLDER_PADDING + row as f32 * ( tile_size + TILE_GAP );
			tiles.push( FolderTileRegion { address: FolderTileAddress { owner, tile_index }, rect: D2D_RECT_F { left: tile_left, top: tile_top, right: tile_left + tile_size, bottom: tile_top + tile_size } } );
		}
		self.folder_panel = Some( FolderPanelRegion { owner, rect, tiles } );
	}
}


pub fn resolved_tile_positions( bar: &TileBar, tiles_per_row: usize ) -> Vec< TilePosition > {
	let units_per_row = tiles_per_row.max( 1 ) * 2;
	let mut occupied = BTreeSet::new();
	let mut positions = Vec::with_capacity( bar.tiles.len() );
	for ( tile_index, tile ) in bar.tiles.iter().enumerate() {
		let legacy = TilePosition { column: ( tile_index % tiles_per_row.max( 1 ) * 2 ) as u8, row: ( tile_index / tiles_per_row.max( 1 ) * 2 ).min( u16::MAX as usize ) as u16 };
		let preferred = tile.grid_position.or_else( || tile.position.map( |position| TilePosition { column: position.column.saturating_mul( 2 ), row: position.row.saturating_mul( 2 ) } ) ).unwrap_or( legacy );
		let position = find_free_position( preferred, tile.size.grid_width(), tile.size.grid_height(), units_per_row, &occupied );
		occupy( position, tile.size.grid_width(), tile.size.grid_height(), &mut occupied );
		positions.push( position );
	}
	positions
}


fn tile_position( bar: &BarRegion, x: f32, y: f32, grid_width: usize ) -> TilePosition {
	let step = ( bar.unit_size + TILE_GAP ).max( 1.0 );
	let column = ( ( x - bar.tile_origin_x ) / step ).round().clamp( 0.0, bar.units_per_row.saturating_sub( grid_width.min( bar.units_per_row ).max( 1 ) ) as f32 ) as u8;
	let row = ( ( y - bar.tile_origin_y ) / step ).round().clamp( 0.0, u16::MAX as f32 ) as u16;
	TilePosition { column, row }
}


fn area_tile_position( area: &BarAreaRegion, x: f32, grid_width: usize ) -> TilePosition {
	let step = ( area.unit_size + TILE_GAP ).max( 1.0 );
	let column = ( ( x - area.tile_origin_x ) / step ).round().clamp( 0.0, area.units_per_row.saturating_sub( grid_width.min( area.units_per_row ).max( 1 ) ) as f32 ) as u8;
	TilePosition { column, row: 0 }
}


fn find_free_position( preferred: TilePosition, width: usize, height: usize, units_per_row: usize, occupied: &BTreeSet< ( usize, usize ) > ) -> TilePosition {
	let width = width.min( units_per_row ).max( 1 );
	let mut slot = preferred.row as usize * units_per_row + ( preferred.column as usize ).min( units_per_row - width );
	loop {
		let column = slot % units_per_row;
		let row = slot / units_per_row;
		if column + width <= units_per_row && ( 0..height ).all( |y| ( 0..width ).all( |x| !occupied.contains( &( column + x, row + y ) ) ) ) { return TilePosition { column: column as u8, row: row.min( u16::MAX as usize ) as u16 }; }
		slot += 1;
	}
}


fn occupy( position: TilePosition, width: usize, height: usize, occupied: &mut BTreeSet< ( usize, usize ) > ) {
	for y in 0..height { for x in 0..width { occupied.insert( ( position.column as usize + x, position.row as usize + y ) ); } }
}


fn expanded( rect: &D2D_RECT_F, amount: f32 ) -> D2D_RECT_F {
	D2D_RECT_F { left: rect.left - amount, top: rect.top - amount, right: rect.right + amount, bottom: rect.bottom + amount }
}


fn contains( rect: &D2D_RECT_F, x: f32, y: f32 ) -> bool {
	x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
}


fn distance_squared( rect: &D2D_RECT_F, x: f32, y: f32 ) -> f32 {
	let center_x = ( rect.left + rect.right ) * 0.5;
	let center_y = ( rect.top + rect.bottom ) * 0.5;
	( center_x - x ).powi( 2 ) + ( center_y - y ).powi( 2 )
}


pub fn interpolate_rect( from: D2D_RECT_F, to: D2D_RECT_F, progress: f32 ) -> D2D_RECT_F {
	D2D_RECT_F { left: from.left + ( to.left - from.left ) * progress, top: from.top + ( to.top - from.top ) * progress, right: from.right + ( to.right - from.right ) * progress, bottom: from.bottom + ( to.bottom - from.bottom ) * progress }
}


pub fn smooth_step( value: f32 ) -> f32 {
	value * value * ( 3.0 - 2.0 * value )
}


pub fn reflow_ease( value: f32 ) -> f32 {
	ease_out_back( value, 0.55 )
}


pub fn indicator_ease( value: f32 ) -> f32 {
	ease_out_back( value, 1.05 )
}


fn ease_out_back( value: f32, strength: f32 ) -> f32 {
	if value <= 0.0 { return 0.0; }
	if value >= 1.0 { return 1.0; }
	let offset = value - 1.0;
	1.0 + ( strength + 1.0 ) * offset * offset * offset + strength * offset * offset
}


#[cfg( test )]
mod tests {
	use super::*;
	use crate::config::Tile;


	#[test]
	fn explicit_tile_gap_is_preserved() {
		let mut tiles = ( 0..3 ).map( tile ).collect::< Vec< _ > >();
		tiles[ 1 ].grid_position = Some( TilePosition { column: 6, row: 2 } );
		let bar = TileBar { title: "bar".to_string(), column: None, locked: false, tiles };
		assert_eq!( resolved_tile_positions( &bar, 4 ), vec![ TilePosition { column: 0, row: 0 }, TilePosition { column: 6, row: 2 }, TilePosition { column: 4, row: 0 } ] );
	}


	#[test]
	fn bars_stack_vertically_inside_the_same_column() {
		let config = StartConfig { bars: vec![ bar( "first", 2 ), bar( "second", 6 ) ] };
		let preferences = preferences();
		let mut layout = TileLayout::default();
		layout.calculate( 1200.0, 800.0, &config, &preferences, None );
		assert_eq!( layout.bars[ 0 ].column, 0 );
		assert_eq!( layout.bars[ 1 ].column, 0 );
		assert!( layout.bars[ 1 ].rect.top > layout.bars[ 0 ].rect.bottom );
	}


	#[test]
	fn dragged_tile_position_preserves_the_pointer_grab_offset() {
		let config = StartConfig { bars: vec![ bar( "first", 1 ) ] };
		let preferences = preferences();
		let mut layout = TileLayout::default();
		layout.calculate( 1200.0, 800.0, &config, &preferences, None );
		let tile = layout.tiles[ 0 ];
		let bar = layout.bars[ 0 ];
		let grab_offset_x = ( tile.rect.right - tile.rect.left ) * 0.75;
		let grab_offset_y = ( tile.rect.bottom - tile.rect.top ) * 0.5;
		let step = bar.unit_size + TILE_GAP;
		let target = layout.dragged_tile_drop_target( tile.rect.left + grab_offset_x + step * 2.0, tile.rect.top + grab_offset_y, grab_offset_x, grab_offset_y, 2 );
		assert_eq!( target, Some( DropTarget::Tile { bar_index: 0, position: TilePosition { column: 2, row: 0 } } ) );
	}


	fn preferences() -> StartPreferences {
		StartPreferences { overlay_opacity_percent: 50, blur_percent: 0, opening_duration_ms: 350, shortcut: configuration::StartShortcut::WinShift, open_on_start_button_click: true, rounded_tiles: true, rounded_tile_bars: true, tile_animation_duration_ms: 220, tile_background_opacity_percent: 64, tile_bar_background_opacity_percent: 64, tile_bar_columns: 1, tiles_per_row: 4 }
	}


	fn bar( title: &str, count: usize ) -> TileBar {
		TileBar { title: title.to_string(), column: Some( 0 ), locked: false, tiles: ( 0..count ).map( tile ).collect() }
	}


	fn tile( index: usize ) -> Tile {
		Tile { runtime_id: crate::config::next_tile_runtime_id(), title: index.to_string(), position: None, grid_position: None, size: crate::config::TileSize::Normal, target: "test.exe".to_string(), arguments: String::new(), working_directory: String::new(), color: "#0067C0".to_string(), icon_source: String::new(), tiles: Vec::new() }
	}
}
