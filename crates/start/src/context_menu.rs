//! ::  Project Path  ->  ep_start :: context_menu.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/23 13:24 周二


use std::time::{ Duration, Instant };
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;


const MENU_WIDTH: f32 = 220.0;
const MENU_PADDING: f32 = 6.0;
const MENU_ROW_HEIGHT: f32 = 40.0;
const MENU_SEPARATOR_MARGIN: f32 = 6.0;
const MENU_SEPARATOR_HEIGHT: f32 = 1.0;
const MENU_GAP: f32 = 4.0;
const VIEWPORT_MARGIN: f32 = 8.0;
const OPEN_ANIMATION_DURATION: Duration = Duration::from_millis( 120 );


pub( crate ) struct ContextMenuItem {
	pub( crate ) id: u32,
	pub( crate ) label: String,
	pub( crate ) icon: String,
	pub( crate ) children: Vec< ContextMenuNode >,
}


pub( crate ) enum ContextMenuNode {
	Item( ContextMenuItem ),
	Separator,
}


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub( crate ) enum ContextMenuInteraction {
	KeepOpen,
	Dismiss,
	Command( u32 ),
}


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub( crate ) enum ContextMenuPanel {
	Root,
	Submenu,
}


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub( crate ) struct ContextMenuSelection {
	pub( crate ) panel: ContextMenuPanel,
	pub( crate ) item_index: usize,
}


#[derive( Clone, Copy )]
pub( crate ) struct ContextMenuRowLayout {
	pub( crate ) item_index: usize,
	pub( crate ) rect: D2D_RECT_F,
}


pub( crate ) struct ContextMenuPanelLayout {
	pub( crate ) panel: D2D_RECT_F,
	pub( crate ) rows: Vec< ContextMenuRowLayout >,
	pub( crate ) separators: Vec< D2D_RECT_F >,
}


struct OpenSubmenu {
	parent_index: usize,
	layout: ContextMenuPanelLayout,
	opened_at: Instant,
}


pub( crate ) struct ContextMenu {
	items: Vec< ContextMenuNode >,
	root_layout: ContextMenuPanelLayout,
	submenu: Option< OpenSubmenu >,
	hovered: Option< ContextMenuSelection >,
	pressed: Option< ContextMenuSelection >,
	opened_at: Instant,
	viewport_width: f32,
	viewport_height: f32,
}


#[derive( Clone, Copy )]
pub( crate ) struct ContextMenuPanelVisual<'a> {
	pub( crate ) items: &'a [ ContextMenuNode ],
	pub( crate ) layout: &'a ContextMenuPanelLayout,
	pub( crate ) panel: ContextMenuPanel,
	pub( crate ) progress: f32,
}


#[derive( Clone, Copy )]
pub( crate ) struct ContextMenuVisual<'a> {
	pub( crate ) root: ContextMenuPanelVisual<'a>,
	pub( crate ) submenu: Option< ContextMenuPanelVisual<'a> >,
	pub( crate ) hovered: Option< ContextMenuSelection >,
	pub( crate ) pressed: Option< ContextMenuSelection >,
}


impl ContextMenuItem {
	pub( crate ) fn command( id: u32, label: impl Into< String >, icon: impl Into< String > ) -> ContextMenuNode {
		ContextMenuNode::Item( Self { id, label: label.into(), icon: icon.into(), children: Vec::new() } )
	}


	pub( crate ) fn submenu( label: impl Into< String >, icon: impl Into< String >, children: Vec< ContextMenuNode > ) -> ContextMenuNode {
		ContextMenuNode::Item( Self { id: 0, label: label.into(), icon: icon.into(), children } )
	}
}


impl ContextMenu {
	pub( crate ) fn open( x: f32, y: f32, viewport_width: f32, viewport_height: f32, items: Vec< ContextMenuNode > ) -> Self {
		let root_layout = ContextMenuPanelLayout::root( x, y, viewport_width, viewport_height, &items );
		Self { items, root_layout, submenu: None, hovered: None, pressed: None, opened_at: Instant::now(), viewport_width, viewport_height }
	}


	pub( crate ) fn pointer_move( &mut self, x: f32, y: f32 ) -> bool {
		let hovered = self.hit_test( x, y );
		let previous_submenu = self.submenu.as_ref().map( |submenu| submenu.parent_index );
		if let Some( ContextMenuSelection { panel: ContextMenuPanel::Root, item_index } ) = hovered {
			if self.item( ContextMenuPanel::Root, item_index ).is_some_and( |item| !item.children.is_empty() ) { self.open_submenu( item_index ); } else { self.submenu = None; }
		}
		let changed = self.hovered != hovered || previous_submenu != self.submenu.as_ref().map( |submenu| submenu.parent_index );
		self.hovered = hovered;
		changed
	}


	pub( crate ) fn pointer_down( &mut self, x: f32, y: f32 ) -> ContextMenuInteraction {
		let Some( selection ) = self.hit_test( x, y ) else { return ContextMenuInteraction::Dismiss; };
		self.hovered = Some( selection );
		if self.item( selection.panel, selection.item_index ).is_some_and( |item| !item.children.is_empty() ) {
			if selection.panel == ContextMenuPanel::Root { self.open_submenu( selection.item_index ); }
			self.pressed = None;
		} else { self.pressed = Some( selection ); }
		ContextMenuInteraction::KeepOpen
	}


	pub( crate ) fn pointer_up( &mut self, x: f32, y: f32 ) -> ContextMenuInteraction {
		let hit = self.hit_test( x, y );
		let pressed = self.pressed.take();
		if hit.is_none() && !self.contains( x, y ) { return ContextMenuInteraction::Dismiss; }
		let Some( selection ) = hit.filter( |selection| Some( *selection ) == pressed ) else { return ContextMenuInteraction::KeepOpen; };
		let Some( item ) = self.item( selection.panel, selection.item_index ) else { return ContextMenuInteraction::KeepOpen; };
		if item.id == 0 { ContextMenuInteraction::KeepOpen } else { ContextMenuInteraction::Command( item.id ) }
	}


	pub( crate ) fn pointer_leave( &mut self ) -> bool {
		let changed = self.hovered.take().is_some() || self.pressed.take().is_some();
		changed
	}


	pub( crate ) fn visual( &self ) -> ContextMenuVisual<'_> {
		let root = ContextMenuPanelVisual { items: &self.items, layout: &self.root_layout, panel: ContextMenuPanel::Root, progress: animation_progress( self.opened_at ) };
		let submenu = self.submenu.as_ref().and_then( |submenu| match self.items.get( submenu.parent_index ) { Some( ContextMenuNode::Item( item ) ) => Some( ContextMenuPanelVisual { items: &item.children, layout: &submenu.layout, panel: ContextMenuPanel::Submenu, progress: animation_progress( submenu.opened_at ) } ), _ => None } );
		ContextMenuVisual { root, submenu, hovered: self.hovered, pressed: self.pressed }
	}


	pub( crate ) fn is_animating( &self ) -> bool {
		animation_progress( self.opened_at ) < 1.0 || self.submenu.as_ref().is_some_and( |submenu| animation_progress( submenu.opened_at ) < 1.0 )
	}


	fn open_submenu( &mut self, parent_index: usize ) {
		if self.submenu.as_ref().is_some_and( |submenu| submenu.parent_index == parent_index ) { return; }
		let Some( row ) = self.root_layout.rows.iter().find( |row| row.item_index == parent_index ).copied() else { return; };
		let Some( item ) = self.item( ContextMenuPanel::Root, parent_index ) else { return; };
		if item.children.is_empty() { return; }
		let layout = ContextMenuPanelLayout::submenu( self.root_layout.panel, row.rect, self.viewport_width, self.viewport_height, &item.children );
		self.submenu = Some( OpenSubmenu { parent_index, layout, opened_at: Instant::now() } );
	}


	fn hit_test( &self, x: f32, y: f32 ) -> Option< ContextMenuSelection > {
		if let Some( submenu ) = &self.submenu {
			if let Some( row ) = submenu.layout.rows.iter().find( |row| rect_contains( row.rect, x, y ) ) { return Some( ContextMenuSelection { panel: ContextMenuPanel::Submenu, item_index: row.item_index } ); }
		}
		self.root_layout.rows.iter().find( |row| rect_contains( row.rect, x, y ) ).map( |row| ContextMenuSelection { panel: ContextMenuPanel::Root, item_index: row.item_index } )
	}


	fn contains( &self, x: f32, y: f32 ) -> bool {
		rect_contains( self.root_layout.panel, x, y ) || self.submenu.as_ref().is_some_and( |submenu| rect_contains( submenu.layout.panel, x, y ) )
	}


	fn item( &self, panel: ContextMenuPanel, item_index: usize ) -> Option< &ContextMenuItem > {
		let items = match panel {
			ContextMenuPanel::Root => &self.items,
			ContextMenuPanel::Submenu => {
				let submenu = self.submenu.as_ref()?;
				let ContextMenuNode::Item( parent ) = self.items.get( submenu.parent_index )? else { return None; };
				&parent.children
			}
		};
		match items.get( item_index ) { Some( ContextMenuNode::Item( item ) ) => Some( item ), _ => None }
	}
}


impl ContextMenuPanelLayout {
	fn root( anchor_x: f32, anchor_y: f32, viewport_width: f32, viewport_height: f32, items: &[ ContextMenuNode ] ) -> Self {
		let height = panel_height( items );
		let left = clamped_origin( anchor_x, MENU_WIDTH, viewport_width );
		let top = clamped_origin( anchor_y, height, viewport_height );
		Self::calculate( left, top, items )
	}


	fn submenu( root: D2D_RECT_F, parent_row: D2D_RECT_F, viewport_width: f32, viewport_height: f32, items: &[ ContextMenuNode ] ) -> Self {
		let height = panel_height( items );
		let preferred_left = root.right + MENU_GAP;
		let left = if preferred_left + MENU_WIDTH <= viewport_width - VIEWPORT_MARGIN { preferred_left } else { root.left - MENU_GAP - MENU_WIDTH };
		let left = clamped_origin( left, MENU_WIDTH, viewport_width );
		let top = clamped_origin( parent_row.top, height, viewport_height );
		Self::calculate( left, top, items )
	}


	fn calculate( left: f32, top: f32, items: &[ ContextMenuNode ] ) -> Self {
		let panel = rect_from_size( left, top, MENU_WIDTH, panel_height( items ) );
		let mut rows = Vec::new();
		let mut separators = Vec::new();
		let mut cursor = top + MENU_PADDING;
		for ( item_index, item ) in items.iter().enumerate() {
			match item {
				ContextMenuNode::Item( _ ) => {
					rows.push( ContextMenuRowLayout { item_index, rect: rect_from_size( left + MENU_PADDING, cursor, MENU_WIDTH - MENU_PADDING * 2.0, MENU_ROW_HEIGHT ) } );
					cursor += MENU_ROW_HEIGHT;
				}
				ContextMenuNode::Separator => {
					cursor += MENU_SEPARATOR_MARGIN;
					separators.push( rect_from_size( left + 12.0, cursor, MENU_WIDTH - 24.0, MENU_SEPARATOR_HEIGHT ) );
					cursor += MENU_SEPARATOR_HEIGHT + MENU_SEPARATOR_MARGIN;
				}
			}
		}
		Self { panel, rows, separators }
	}
}


fn panel_height( items: &[ ContextMenuNode ] ) -> f32 {
	MENU_PADDING * 2.0 + items.iter().map( |item| match item { ContextMenuNode::Item( _ ) => MENU_ROW_HEIGHT, ContextMenuNode::Separator => MENU_SEPARATOR_MARGIN * 2.0 + MENU_SEPARATOR_HEIGHT } ).sum::< f32 >()
}


fn animation_progress( opened_at: Instant ) -> f32 {
	( opened_at.elapsed().as_secs_f32() / OPEN_ANIMATION_DURATION.as_secs_f32() ).clamp( 0.0, 1.0 )
}


fn clamped_origin( value: f32, size: f32, viewport_size: f32 ) -> f32 {
	let maximum = ( viewport_size - size - VIEWPORT_MARGIN ).max( 0.0 );
	value.clamp( VIEWPORT_MARGIN.min( maximum ), maximum )
}


fn rect_from_size( left: f32, top: f32, width: f32, height: f32 ) -> D2D_RECT_F {
	D2D_RECT_F { left, top, right: left + width, bottom: top + height }
}


fn rect_contains( rect: D2D_RECT_F, x: f32, y: f32 ) -> bool {
	x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
}


#[cfg( test )]
mod tests {
	use super::*;


	fn menu_items() -> Vec< ContextMenuNode > {
		vec![ ContextMenuItem::submenu( "新建磁贴", "", vec![ ContextMenuItem::command( 1, "小", "" ), ContextMenuItem::command( 2, "正常", "" ) ] ), ContextMenuNode::Separator, ContextMenuItem::command( 3, "锁定磁贴栏", "" ) ]
	}


	#[test]
	fn menu_stays_inside_viewport() {
		let mut menu = ContextMenu::open( 1900.0, 1040.0, 1920.0, 1080.0, menu_items() );
		let row = menu.root_layout.rows[ 0 ].rect;
		menu.pointer_move( row.left + 10.0, row.top + 10.0 );
		assert!( menu.root_layout.panel.right <= 1920.0 );
		assert!( menu.root_layout.panel.bottom <= 1080.0 );
		assert!( menu.submenu.as_ref().unwrap().layout.panel.left >= 0.0 );
		assert!( menu.submenu.as_ref().unwrap().layout.panel.bottom <= 1080.0 );
	}


	#[test]
	fn submenu_opens_when_parent_is_hovered() {
		let mut menu = ContextMenu::open( 100.0, 100.0, 1920.0, 1080.0, menu_items() );
		let row = menu.root_layout.rows[ 0 ].rect;
		assert!( menu.pointer_move( row.left + 10.0, row.top + 10.0 ) );
		assert!( menu.submenu.is_some() );
	}


	#[test]
	fn submenu_returns_registered_command() {
		let mut menu = ContextMenu::open( 100.0, 100.0, 1920.0, 1080.0, menu_items() );
		let parent = menu.root_layout.rows[ 0 ].rect;
		menu.pointer_move( parent.left + 10.0, parent.top + 10.0 );
		let row = menu.submenu.as_ref().unwrap().layout.rows[ 1 ].rect;
		let x = row.left + 10.0;
		let y = row.top + 10.0;
		assert_eq!( menu.pointer_down( x, y ), ContextMenuInteraction::KeepOpen );
		assert_eq!( menu.pointer_up( x, y ), ContextMenuInteraction::Command( 2 ) );
	}
}
