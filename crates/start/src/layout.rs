//! ::  Project Path  ->  ep_start :: layout.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 21:40 周六


use crate::config::StartConfig;
use configuration::StartPreferences;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;


const CONTENT_MARGIN_X: f32 = 72.0;
const CONTENT_MARGIN_Y: f32 = 44.0;
const BAR_TITLE_HEIGHT: f32 = 34.0;
const BAR_TITLE_GAP: f32 = 8.0;
const BAR_GAP: f32 = 34.0;
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
pub struct BarRegion {
	pub bar_index: usize,
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
	Tile { bar_index: usize, slot_index: usize },
	Bar( usize ),
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
	pub bars: Vec< BarRegion >,
	pub tiles: Vec< TileRegion >,
	pub folder_panel: Option< FolderPanelRegion >,
}


impl TileLayout {
	pub fn calculate( &mut self, client_width: f32, client_height: f32, config: &StartConfig, preferences: &StartPreferences, open_folder: Option< TileAddress > ) {
		self.bars.clear();
		self.tiles.clear();
		self.folder_panel = None;
		let bar_columns = preferences.tile_bar_columns.clamp( 1, 6 ) as usize;
		let tiles_per_row = preferences.tiles_per_row.clamp( 3, 5 ) as usize;
		let bar_rows = config.bars.len().div_ceil( bar_columns ).max( 1 );
		let available_width = ( client_width - CONTENT_MARGIN_X * 2.0 - BAR_GAP * bar_columns.saturating_sub( 1 ) as f32 ).max( 1.0 );
		let available_height = ( client_height - CONTENT_MARGIN_Y * 2.0 - BAR_GAP * bar_rows.saturating_sub( 1 ) as f32 ).max( 1.0 );
		let bar_width = available_width / bar_columns as f32;
		let bar_height = available_height / bar_rows as f32;
		for ( bar_index, bar ) in config.bars.iter().enumerate() {
			let bar_column = bar_index % bar_columns;
			let bar_row = bar_index / bar_columns;
			let left = CONTENT_MARGIN_X + bar_column as f32 * ( bar_width + BAR_GAP );
			let top = CONTENT_MARGIN_Y + bar_row as f32 * ( bar_height + BAR_GAP );
			let rect = D2D_RECT_F { left, top, right: ( left + bar_width ).min( client_width - CONTENT_MARGIN_X ), bottom: ( top + bar_height ).min( client_height - CONTENT_MARGIN_Y ) };
			let title_rect = D2D_RECT_F { left, top, right: rect.right, bottom: top + BAR_TITLE_HEIGHT };
			let tile_rows = bar.tiles.len().div_ceil( tiles_per_row ).max( 1 );
			let width_size = ( bar_width - TILE_GAP * tiles_per_row.saturating_sub( 1 ) as f32 ) / tiles_per_row as f32;
			let tile_area_height = ( bar_height - BAR_TITLE_HEIGHT - BAR_TITLE_GAP ).max( 1.0 );
			let height_size = ( tile_area_height - TILE_GAP * tile_rows.saturating_sub( 1 ) as f32 ) / tile_rows as f32;
			let tile_size = width_size.min( height_size ).max( 1.0 );
			let tile_origin_y = top + BAR_TITLE_HEIGHT + BAR_TITLE_GAP;
			self.bars.push( BarRegion { bar_index, rect, title_rect, tile_origin_x: left, tile_origin_y, tile_size, tiles_per_row } );
			for ( tile_index, _ ) in bar.tiles.iter().enumerate() {
				let column = tile_index % tiles_per_row;
				let row = tile_index / tiles_per_row;
				let tile_left = left + column as f32 * ( tile_size + TILE_GAP );
				let tile_top = tile_origin_y + row as f32 * ( tile_size + TILE_GAP );
				self.tiles.push( TileRegion { address: TileAddress { bar_index, tile_index }, rect: D2D_RECT_F { left: tile_left, top: tile_top, right: ( tile_left + tile_size ).min( rect.right ), bottom: ( tile_top + tile_size ).min( rect.bottom ) } } );
			}
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


	pub fn nearest_bar( &self, x: f32, y: f32 ) -> Option< usize > {
		self.bars.iter().min_by( |left, right| distance_squared( &left.rect, x, y ).total_cmp( &distance_squared( &right.rect, x, y ) ) ).map( |bar| bar.bar_index )
	}


	pub fn tile_slot_index( &self, bar_index: usize, x: f32, y: f32, tile_count: usize ) -> usize {
		let Some( bar ) = self.bars.iter().find( |bar| bar.bar_index == bar_index ) else { return 0; };
		let step = ( bar.tile_size + TILE_GAP ).max( 1.0 );
		let column = ( ( x - bar.tile_origin_x ) / step ).round().clamp( 0.0, bar.tiles_per_row.saturating_sub( 1 ) as f32 ) as usize;
		let row = ( ( y - bar.tile_origin_y ) / step ).round().max( 0.0 ) as usize;
		( row * bar.tiles_per_row + column ).min( tile_count )
	}


	pub fn tile_rect( &self, address: TileAddress ) -> Option< D2D_RECT_F > {
		self.tiles.iter().find( |tile| tile.address == address ).map( |tile| tile.rect )
	}


	pub fn bar_rect( &self, bar_index: usize ) -> Option< D2D_RECT_F > {
		self.bars.iter().find( |bar| bar.bar_index == bar_index ).map( |bar| bar.rect )
	}


	pub fn drop_rect( &self, target: DropTarget ) -> Option< D2D_RECT_F > {
		match target {
			DropTarget::Bar( bar_index ) => self.bar_rect( bar_index ),
			DropTarget::Tile { bar_index, slot_index } => self.tile_slot_rect( bar_index, slot_index ),
		}
	}


	fn tile_slot_rect( &self, bar_index: usize, slot_index: usize ) -> Option< D2D_RECT_F > {
		let bar = self.bars.iter().find( |bar| bar.bar_index == bar_index )?;
		let column = slot_index % bar.tiles_per_row;
		let row = slot_index / bar.tiles_per_row;
		let left = bar.tile_origin_x + column as f32 * ( bar.tile_size + TILE_GAP );
		let top = bar.tile_origin_y + row as f32 * ( bar.tile_size + TILE_GAP );
		Some( D2D_RECT_F { left, top, right: left + bar.tile_size, bottom: top + bar.tile_size } )
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
	use crate::config::{ Tile, TileBar };


	#[test]
	fn tile_slot_uses_rows_and_columns() {
		let config = StartConfig { bars: vec![ TileBar { title: "bar".to_string(), tiles: ( 0..8 ).map( tile ).collect() } ] };
		let preferences = StartPreferences { overlay_opacity_percent: 50, blur_percent: 0, opening_duration_ms: 350, tile_bar_columns: 1, tiles_per_row: 4 };
		let mut layout = TileLayout::default();
		layout.calculate( 1200.0, 800.0, &config, &preferences, None );
		let first = layout.tile_rect( TileAddress { bar_index: 0, tile_index: 0 } ).unwrap();
		let fifth = layout.tile_rect( TileAddress { bar_index: 0, tile_index: 4 } ).unwrap();
		assert_eq!( layout.tile_slot_index( 0, center( first ).0, center( first ).1, 8 ), 0 );
		assert_eq!( layout.tile_slot_index( 0, center( fifth ).0, center( fifth ).1, 8 ), 4 );
	}


	fn tile( index: usize ) -> Tile {
		Tile { title: index.to_string(), target: "test.exe".to_string(), arguments: String::new(), working_directory: String::new(), color: "#0067C0".to_string(), tiles: Vec::new() }
	}


	fn center( rect: D2D_RECT_F ) -> ( f32, f32 ) {
		( ( rect.left + rect.right ) * 0.5, ( rect.top + rect.bottom ) * 0.5 )
	}
}
