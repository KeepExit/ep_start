//! ::  Project Path  ->  ep_start :: layout.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 21:40 周六


use crate::config::{ StartConfig, TileBar, TilePosition };
use configuration::StartPreferences;
use std::collections::BTreeSet;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;


const CONTENT_MARGIN_X: f32 = 72.0;
const CONTENT_MARGIN_Y: f32 = 44.0;
const BAR_TITLE_HEIGHT: f32 = 34.0;
const BAR_TITLE_GAP: f32 = 8.0;
const BAR_COLUMN_GAP: f32 = 30.0;
const BAR_STACK_GAP: f32 = 28.0;
const BAR_PADDING_X: f32 = 12.0;
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
}


#[derive( Clone, Copy )]
pub struct BarRegion {
	pub bar_index: usize,
	pub column: usize,
	pub rect: D2D_RECT_F,
	pub title_rect: D2D_RECT_F,
	pub tile_origin_x: f32,
	pub tile_origin_y: f32,
	pub tile_size: f32,
	pub tiles_per_row: usize,
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
	NewBar { column: usize, stack_index: usize },
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
		let available_width = ( client_width - CONTENT_MARGIN_X * 2.0 - BAR_COLUMN_GAP * bar_columns.saturating_sub( 1 ) as f32 ).max( 1.0 );
		let area_width = available_width / bar_columns as f32;
		let tile_size = ( ( area_width - BAR_PADDING_X * 2.0 - TILE_GAP * tiles_per_row.saturating_sub( 1 ) as f32 ) / tiles_per_row as f32 ).max( 1.0 );
		let mut column_tops = vec![ CONTENT_MARGIN_Y; bar_columns ];
		for column in 0..bar_columns {
			let left = CONTENT_MARGIN_X + column as f32 * ( area_width + BAR_COLUMN_GAP );
			self.areas.push( BarAreaRegion { column, rect: D2D_RECT_F { left, top: CONTENT_MARGIN_Y, right: left + area_width, bottom: ( client_height - CONTENT_MARGIN_Y ).max( CONTENT_MARGIN_Y ) } } );
		}
		for ( bar_index, bar ) in config.bars.iter().enumerate() {
			let column = bar.column.map( usize::from ).unwrap_or( bar_index % bar_columns ).min( bar_columns - 1 );
			let area = self.areas[ column ];
			let slots = resolved_tile_slots( bar, tiles_per_row );
			let rows = slots.iter().max().map( |slot| slot / tiles_per_row + 1 ).unwrap_or( 1 );
			let top = column_tops[ column ];
			let tile_origin_y = top + BAR_TITLE_HEIGHT + BAR_TITLE_GAP;
			let tile_height = rows as f32 * tile_size + rows.saturating_sub( 1 ) as f32 * TILE_GAP;
			let rect = D2D_RECT_F { left: area.rect.left, top, right: area.rect.right, bottom: tile_origin_y + tile_height + BAR_PADDING_BOTTOM };
			let title_rect = D2D_RECT_F { left: rect.left + BAR_PADDING_X, top, right: rect.right - BAR_PADDING_X, bottom: top + BAR_TITLE_HEIGHT };
			let tile_origin_x = rect.left + BAR_PADDING_X;
			self.bars.push( BarRegion { bar_index, column, rect, title_rect, tile_origin_x, tile_origin_y, tile_size, tiles_per_row } );
			for ( tile_index, slot ) in slots.into_iter().enumerate() {
				let position = TilePosition { column: ( slot % tiles_per_row ) as u8, row: ( slot / tiles_per_row ).min( u16::MAX as usize ) as u16 };
				let tile_left = tile_origin_x + position.column as f32 * ( tile_size + TILE_GAP );
				let tile_top = tile_origin_y + position.row as f32 * ( tile_size + TILE_GAP );
				self.tiles.push( TileRegion { address: TileAddress { bar_index, tile_index }, rect: D2D_RECT_F { left: tile_left, top: tile_top, right: tile_left + tile_size, bottom: tile_top + tile_size } } );
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


	pub fn tile_drop_target( &self, x: f32, y: f32 ) -> Option< DropTarget > {
		let bar = self.bars.iter().filter( |bar| contains( &expanded( &bar.rect, bar.tile_size * BAR_HIT_EXPANSION_RATIO ), x, y ) ).min_by( |left, right| distance_squared( &left.rect, x, y ).total_cmp( &distance_squared( &right.rect, x, y ) ) );
		if let Some( bar ) = bar { return Some( DropTarget::Tile { bar_index: bar.bar_index, position: tile_position( bar, x, y ) } ); }
		let area = self.areas.iter().find( |area| contains( &area.rect, x, y ) )?;
		Some( DropTarget::NewBar { column: area.column, stack_index: self.stack_index( area.column, y ) } )
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
			DropTarget::Tile { bar_index, position } => self.tile_position_rect( bar_index, position ),
			DropTarget::NewBar { column, stack_index } | DropTarget::Bar { column, stack_index } => self.stack_drop_rect( column, stack_index ),
		}
	}


	fn tile_position_rect( &self, bar_index: usize, position: TilePosition ) -> Option< D2D_RECT_F > {
		let bar = self.bars.iter().find( |bar| bar.bar_index == bar_index )?;
		let left = bar.tile_origin_x + position.column.min( bar.tiles_per_row.saturating_sub( 1 ) as u8 ) as f32 * ( bar.tile_size + TILE_GAP );
		let top = bar.tile_origin_y + position.row as f32 * ( bar.tile_size + TILE_GAP );
		Some( D2D_RECT_F { left, top, right: left + bar.tile_size, bottom: top + bar.tile_size } )
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


pub fn resolved_tile_slots( bar: &TileBar, tiles_per_row: usize ) -> Vec< usize > {
	let tiles_per_row = tiles_per_row.max( 1 );
	let mut occupied = BTreeSet::new();
	let mut slots = Vec::with_capacity( bar.tiles.len() );
	for ( tile_index, tile ) in bar.tiles.iter().enumerate() {
		let mut slot = tile.position.map( |position| position.row as usize * tiles_per_row + position.column.min( tiles_per_row.saturating_sub( 1 ) as u8 ) as usize ).unwrap_or( tile_index );
		while occupied.contains( &slot ) { slot += 1; }
		occupied.insert( slot );
		slots.push( slot );
	}
	slots
}


fn tile_position( bar: &BarRegion, x: f32, y: f32 ) -> TilePosition {
	let step = ( bar.tile_size + TILE_GAP ).max( 1.0 );
	let column = ( ( x - bar.tile_origin_x ) / step ).round().clamp( 0.0, bar.tiles_per_row.saturating_sub( 1 ) as f32 ) as u8;
	let row = ( ( y - bar.tile_origin_y ) / step ).round().clamp( 0.0, u16::MAX as f32 ) as u16;
	TilePosition { column, row }
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


#[cfg( test )]
mod tests {
	use super::*;
	use crate::config::Tile;


	#[test]
	fn explicit_tile_gap_is_preserved() {
		let mut tiles = ( 0..3 ).map( tile ).collect::< Vec< _ > >();
		tiles[ 1 ].position = Some( TilePosition { column: 3, row: 1 } );
		let bar = TileBar { title: "bar".to_string(), column: None, tiles };
		assert_eq!( resolved_tile_slots( &bar, 4 ), vec![ 0, 7, 2 ] );
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


	fn preferences() -> StartPreferences {
		StartPreferences { overlay_opacity_percent: 50, blur_percent: 0, opening_duration_ms: 350, shortcut: configuration::StartShortcut::WinShift, open_on_start_button_click: true, tile_bar_columns: 1, tiles_per_row: 4 }
	}


	fn bar( title: &str, count: usize ) -> TileBar {
		TileBar { title: title.to_string(), column: Some( 0 ), tiles: ( 0..count ).map( tile ).collect() }
	}


	fn tile( index: usize ) -> Tile {
		Tile { title: index.to_string(), position: None, target: "test.exe".to_string(), arguments: String::new(), working_directory: String::new(), color: "#0067C0".to_string(), tiles: Vec::new() }
	}
}
