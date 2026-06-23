//! ::  Project Path  ->  ep_start :: layout.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 16:45 周日


use super::action::{ ActionId, ActionLayout };
use super::item::{ SettingId, SettingItem, SettingItemLayout };
use super::section::{ SectionLayout, SettingSection, SettingSubSection, SubSectionLayout };
use super::{ SettingsUi, SidebarItem, SidebarItemId };
use crate::ui::geometry::{ logical_rect, unscale };
use windows::Win32::Foundation::RECT;


const EXPANDED_SIDEBAR_WIDTH: i32 = 246;
const COMPACT_SIDEBAR_WIDTH: i32 = 118;
const CONTENT_MARGIN: i32 = 32;
const HEADER_HEIGHT: i32 = 74;
const FOOTER_HEIGHT: i32 = 68;
const SECTION_HEIGHT: i32 = 38;
const ROW_HEIGHT: i32 = 64;
const STACKED_ROW_HEIGHT: i32 = 128;
const ROW_GAP: i32 = 4;
const SECTION_GAP: i32 = 24;
const BODY_BOTTOM_PADDING: i32 = 28;
const SIDEBAR_MAIN_TOP: i32 = 88;
const SIDEBAR_ITEM_WIDTH: i32 = 256;
const SIDEBAR_ITEM_HEIGHT: i32 = 36;
const SIDEBAR_ITEM_GAP: i32 = 4;
const SIDEBAR_BOTTOM_MARGIN: i32 = 20;
const SCROLLBAR_MARGIN: i32 = 9;
const SCROLLBAR_WIDTH: i32 = 4;
const SCROLLBAR_MIN_THUMB: i32 = 42;

#[derive( Clone, Copy )]
pub( crate ) struct SidebarItemLayout {
	pub( crate ) id: SidebarItemId,
	pub( crate ) area: RECT,
	pub( crate ) icon: &'static str,
	pub( crate ) selected: bool,
}

pub( crate ) struct SidebarLayout {
	pub( crate ) bounds: RECT,
	pub( crate ) title: RECT,
	pub( crate ) main: Vec< SidebarItemLayout >,
	pub( crate ) bottom: Vec< SidebarItemLayout >,
	pub( crate ) expanded: bool,
}

#[derive( Clone, Copy )]
pub( crate ) struct ScrollbarLayout {
	pub( crate ) track: RECT,
	pub( crate ) thumb: RECT,
}

pub( crate ) struct ContentLayout {
	pub( crate ) bounds: RECT,
	pub( crate ) page_title: RECT,
	pub( crate ) viewport: RECT,
	pub( crate ) sections: Vec< SectionLayout >,
	pub( crate ) sub_sections: Vec< SubSectionLayout >,
	pub( crate ) scrollbar: Option< ScrollbarLayout >,
	pub( crate ) scroll_max: i32,
}

pub( crate ) struct FooterLayout {
	pub( crate ) bounds: RECT,
	pub( crate ) actions: Vec< ActionLayout >,
}

pub( crate ) struct SettingsUiLayout {
	pub( crate ) sidebar: SidebarLayout,
	pub( crate ) content: ContentLayout,
	pub( crate ) footer: FooterLayout,
}

impl SettingsUiLayout {
	pub( crate ) fn calculate( ui: &SettingsUi, client: RECT, dpi: i32, scroll_y: i32 ) -> Self {
		let logical_width = unscale( client.right - client.left, dpi );
		let logical_height = unscale( client.bottom - client.top, dpi );
		let expanded = logical_width >= 820;
		let sidebar_width = if expanded { EXPANDED_SIDEBAR_WIDTH } else { COMPACT_SIDEBAR_WIDTH };
		let content_left = sidebar_width + CONTENT_MARGIN;
		let content_right = ( logical_width - CONTENT_MARGIN - 14 ).max( content_left + 280 );
		let content_width = content_right - content_left;
		let stacked = content_width < 620;
		let row_height = if stacked { STACKED_ROW_HEIGHT } else { ROW_HEIGHT };
		let viewport_height = ( logical_height - HEADER_HEIGHT - FOOTER_HEIGHT ).max( 1 );
		let body_height = content_body_height( ui, row_height );
		let scroll_max = ( body_height - viewport_height ).max( 0 );
		let scroll_y = scroll_y.clamp( 0, scroll_max );
		let mut y = HEADER_HEIGHT - scroll_y;
		let sections = ui.content.sections.iter().map( |section| layout_section( section, content_left, content_right, &mut y, row_height, stacked, dpi ) ).collect();
		let sub_sections = ui.content.sub_sections.iter().map( |section| layout_sub_section( section, content_left, content_right, &mut y, row_height, stacked, dpi ) ).collect();
		let sidebar = layout_sidebar( ui, sidebar_width, logical_height, expanded, dpi );
		let content = ContentLayout {
			bounds: logical_rect( sidebar_width, 0, logical_width, logical_height - FOOTER_HEIGHT, dpi ).to_rect(),
			page_title: logical_rect( content_left, 16, content_right, 66, dpi ).to_rect(),
			viewport: logical_rect( sidebar_width, HEADER_HEIGHT, logical_width, logical_height - FOOTER_HEIGHT, dpi ).to_rect(),
			sections,
			sub_sections,
			scrollbar: layout_scrollbar( logical_width, logical_height, viewport_height, body_height, scroll_max, scroll_y, dpi ),
			scroll_max,
		};
		let footer = layout_footer( ui, sidebar_width, logical_width, logical_height, dpi );
		Self { sidebar, content, footer }
	}
	pub( crate ) fn item( &self, id: SettingId ) -> Option< &SettingItemLayout > {
		self.content.sections.iter().flat_map( |section| section.items.iter() ).chain( self.content.sub_sections.iter().flat_map( |section| section.items.iter() ) ).find( |item| item.id == id )
	}
	pub( crate ) fn hit_control( &self, x: i32, y: i32 ) -> Option< SettingId > {
		self.content.sections.iter().flat_map( |section| section.items.iter() ).chain( self.content.sub_sections.iter().flat_map( |section| section.items.iter() ) ).find( |item| contains( item.control, x, y ) ).map( |item| item.id )
	}
	pub( crate ) fn hit_action( &self, x: i32, y: i32 ) -> Option< ActionId > {
		self.footer.actions.iter().find( |action| contains( action.area, x, y ) ).map( |action| action.id )
	}
	pub( crate ) fn hit_scroll_thumb( &self, x: i32, y: i32 ) -> bool {
		self.content.scrollbar.is_some_and( |scrollbar| contains( scrollbar.thumb, x, y ) )
	}
	pub( crate ) fn hit_scroll_track( &self, x: i32, y: i32 ) -> bool {
		self.content.scrollbar.is_some_and( |scrollbar| contains( scrollbar.track, x, y ) )
	}
	pub( crate ) fn scroll_from_thumb( &self, thumb_top: i32 ) -> i32 {
		let Some( scrollbar ) = self.content.scrollbar else { return 0; };
		let travel = ( scrollbar.track.bottom - scrollbar.track.top ) - ( scrollbar.thumb.bottom - scrollbar.thumb.top );
		if travel <= 0 { return 0; }
		( ( thumb_top - scrollbar.track.top ).clamp( 0, travel ) * self.content.scroll_max / travel ).clamp( 0, self.content.scroll_max )
	}
}

fn layout_sidebar( ui: &SettingsUi, width: i32, height: i32, expanded: bool, dpi: i32 ) -> SidebarLayout {
	let item_width = SIDEBAR_ITEM_WIDTH.min( ( width - 16 ).max( 1 ) );
	let item_left = ( width - item_width ) / 2;
	let item_right = item_left + item_width;
	let main = ui.sidebar.main.iter().enumerate().map( |( index, item )| {
		let top = SIDEBAR_MAIN_TOP + index as i32 * ( SIDEBAR_ITEM_HEIGHT + SIDEBAR_ITEM_GAP );
		layout_sidebar_item( *item, item_left, item_right, top, dpi )
	} ).collect();
	let bottom_height = ui.sidebar.bottom.len() as i32 * SIDEBAR_ITEM_HEIGHT + ui.sidebar.bottom.len().saturating_sub( 1 ) as i32 * SIDEBAR_ITEM_GAP;
	let bottom_top = height - SIDEBAR_BOTTOM_MARGIN - bottom_height;
	let bottom = ui.sidebar.bottom.iter().enumerate().map( |( index, item )| {
		let top = bottom_top + index as i32 * ( SIDEBAR_ITEM_HEIGHT + SIDEBAR_ITEM_GAP );
		layout_sidebar_item( *item, item_left, item_right, top, dpi )
	} ).collect();
	SidebarLayout {
		bounds: logical_rect( 0, 0, width, height, dpi ).to_rect(),
		title: logical_rect( 28, 20, width - 14, 66, dpi ).to_rect(),
		main,
		bottom,
		expanded,
	}
}

fn layout_sidebar_item( item: SidebarItem, left: i32, right: i32, top: i32, dpi: i32 ) -> SidebarItemLayout {
	SidebarItemLayout { id: item.id, area: logical_rect( left, top, right, top + SIDEBAR_ITEM_HEIGHT, dpi ).to_rect(), icon: item.icon, selected: item.selected }
}

fn layout_footer( ui: &SettingsUi, left: i32, right: i32, bottom: i32, dpi: i32 ) -> FooterLayout {
	let mut action_right = right - 24;
	let mut actions = Vec::with_capacity( ui.footer.actions.len() );
	for action in ui.footer.actions.iter().rev() {
		let action_left = action_right - 104;
		actions.push( ActionLayout { id: action.id, area: logical_rect( action_left, bottom - 54, action_right, bottom - 14, dpi ).to_rect(), primary: action.primary } );
		action_right = action_left - 10;
	}
	actions.reverse();
	FooterLayout { bounds: logical_rect( left, bottom - FOOTER_HEIGHT, right, bottom, dpi ).to_rect(), actions }
}

fn layout_section( section: &SettingSection, left: i32, right: i32, y: &mut i32, row_height: i32, stacked: bool, dpi: i32 ) -> SectionLayout {
	let top = *y;
	let title = logical_rect( left, *y, right, *y + SECTION_HEIGHT, dpi ).to_rect();
	*y += SECTION_HEIGHT;
	let items = layout_items( &section.items, left, right, y, row_height, stacked, dpi );
	let bottom = if items.is_empty() { *y } else { *y - ROW_GAP };
	*y = bottom + SECTION_GAP;
	SectionLayout { id: section.id, bounds: logical_rect( left, top, right, bottom, dpi ).to_rect(), title, items }
}

fn layout_sub_section( section: &SettingSubSection, left: i32, right: i32, y: &mut i32, row_height: i32, stacked: bool, dpi: i32 ) -> SubSectionLayout {
	let top = *y;
	let items = layout_items( &section.items, left, right, y, row_height, stacked, dpi );
	let bottom = if items.is_empty() { *y } else { *y - ROW_GAP };
	*y = bottom + SECTION_GAP;
	SubSectionLayout { parent: section.parent, bounds: logical_rect( left, top, right, bottom, dpi ).to_rect(), items }
}

fn layout_items( items: &[ SettingItem ], left: i32, right: i32, y: &mut i32, row_height: i32, stacked: bool, dpi: i32 ) -> Vec< SettingItemLayout > {
	items.iter().map( |item| {
		let layout = layout_item( *item, left, right, *y, row_height, stacked, dpi );
		*y += row_height + ROW_GAP;
		layout
	} ).collect()
}

fn layout_item( item: SettingItem, left: i32, right: i32, top: i32, height: i32, stacked: bool, dpi: i32 ) -> SettingItemLayout {
	let card = logical_rect( left, top, right, top + height, dpi ).to_rect();
	let center_y = top + height / 2;
	let icon = logical_rect( left + 18, center_y - 23, left + 64, center_y + 23, dpi ).to_rect();
	let text_left = left + 74;
	let ( title, description, control ) = if stacked {
		let title_top = top + 12;
		let description_top = title_top + 29;
		let control_bottom = top + height - 14;
		(
			logical_rect( text_left, title_top, right - 22, title_top + 31, dpi ).to_rect(),
			logical_rect( text_left, description_top, right - 22, description_top + 31, dpi ).to_rect(),
			logical_rect( text_left, control_bottom - 38, right - 22, control_bottom, dpi ).to_rect(),
		)
	} else {
		let text_right = ( right - 324 ).max( text_left + 120 );
		let control_left = ( right - 300 ).max( text_right + 18 );
		let text_top = center_y - 29;
		(
			logical_rect( text_left, text_top, text_right, text_top + 30, dpi ).to_rect(),
			logical_rect( text_left, text_top + 28, text_right, text_top + 56, dpi ).to_rect(),
			logical_rect( control_left, center_y - 22, right - 22, center_y + 22, dpi ).to_rect(),
		)
	};
	SettingItemLayout { id: item.id, control_kind: item.control, card, icon, title, description, control }
}

fn content_body_height( ui: &SettingsUi, row_height: i32 ) -> i32 {
	let section_height: i32 = ui.content.sections.iter().map( |section| SECTION_HEIGHT + item_block_height( section.items.len(), row_height ) ).sum();
	let sub_section_height: i32 = ui.content.sub_sections.iter().map( |section| item_block_height( section.items.len(), row_height ) ).sum();
	let block_count = ui.content.sections.len() + ui.content.sub_sections.len();
	section_height + sub_section_height + block_count.saturating_sub( 1 ) as i32 * SECTION_GAP + BODY_BOTTOM_PADDING
}

fn item_block_height( count: usize, row_height: i32 ) -> i32 {
	if count == 0 { return 0; }
	count as i32 * row_height + count.saturating_sub( 1 ) as i32 * ROW_GAP
}

fn layout_scrollbar( width: i32, height: i32, viewport_height: i32, body_height: i32, scroll_max: i32, scroll_y: i32, dpi: i32 ) -> Option< ScrollbarLayout > {
	if scroll_max <= 0 { return None; }
	let track_top = HEADER_HEIGHT + 8;
	let track_bottom = ( height - FOOTER_HEIGHT - 8 ).max( track_top + 1 );
	let track_height = track_bottom - track_top;
	let minimum_thumb = SCROLLBAR_MIN_THUMB.min( track_height );
	let thumb_height = ( track_height * viewport_height / body_height.max( 1 ) ).clamp( minimum_thumb, track_height );
	let travel = track_height - thumb_height;
	let thumb_top = track_top + travel * scroll_y / scroll_max;
	Some( ScrollbarLayout {
		track: logical_rect( width - SCROLLBAR_MARGIN - SCROLLBAR_WIDTH, track_top, width - SCROLLBAR_MARGIN, track_bottom, dpi ).to_rect(),
		thumb: logical_rect( width - SCROLLBAR_MARGIN - SCROLLBAR_WIDTH, thumb_top, width - SCROLLBAR_MARGIN, thumb_top + thumb_height, dpi ).to_rect(),
	} )
}

fn contains( rect: RECT, x: i32, y: i32 ) -> bool {
	x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
}


#[cfg( test )]
mod tests {
	use super::*;


	#[test]
	fn minimized_client_size_does_not_break_layout() {
		let ui = SettingsUi::settings_page();
		let layout = SettingsUiLayout::calculate( &ui, RECT::default(), 96, 0 );
		assert!( layout.content.scroll_max >= 0 );
	}
}
