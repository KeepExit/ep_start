//! ::  Project Path  ->  ep_start :: layout.rs :: layout
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 06:44 周日


use windows::Win32::Foundation::RECT;


const EXPANDED_SIDEBAR_WIDTH: i32 = 246;
const COMPACT_SIDEBAR_WIDTH: i32 = 118;
const CONTENT_MARGIN: i32 = 32;
const HEADER_HEIGHT: i32 = 74;
const FOOTER_HEIGHT: i32 = 68;
const SECTION_HEIGHT: i32 = 38;
const GAP_HEIGHT: i32 = 64;
const GAP_BOX_HEIGHT: i32 = 128;
const ROW_GAP: i32 = 4;
const SECTION_GAP: i32 = 24;
const SCROLLBAR_MARGIN: i32 = 9;
const SCROLLBAR_WIDTH: i32 = 4;
const SCROLLBAR_MIN_THUMB: i32 = 42;

#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub(crate) enum SettingId {
	Overlay,
	Blur,
	AnimationDuration,
	BarColumns,
	TilesPerRow,
}

#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub(crate) enum ControlKind {
	Slider,
	Choice,
}

#[derive( Clone, Copy )]
pub(crate) struct SettingRowLayout {
	pub id: SettingId,
	pub kind: ControlKind,
	pub card: RECT,
	pub icon: RECT,
	pub title: RECT,
	pub description: RECT,
	pub control: RECT,
}

#[derive( Clone, Copy )]
pub(crate) struct ScrollbarLayout {
	pub track: RECT,
	pub thumb: RECT,
}

pub(crate) struct SettingsLayout {
	pub sidebar: RECT,
	pub sidebar_nav: RECT,
	pub settings_title: RECT,
	pub page_title: RECT,
	pub menu_section_title: RECT,
	pub tile_section_title: RECT,
	pub undo_button: RECT,
	pub save_button: RECT,
	pub rows: Vec< SettingRowLayout >,
	pub scrollbar: Option< ScrollbarLayout >,
	pub expanded_sidebar: bool,
	pub viewport_top: i32,
	pub viewport_bottom: i32,
	pub scroll_max: i32,
}

impl SettingsLayout {
	pub(crate) fn calculate( client: RECT, dpi: i32, scroll_y: i32 ) -> Self {
		let logical_width = unscale( client.right - client.left, dpi );
		let logical_height = unscale( client.bottom - client.top, dpi );
		let expanded_sidebar = logical_width >= 820;
		let sidebar_width = if expanded_sidebar { EXPANDED_SIDEBAR_WIDTH } else { COMPACT_SIDEBAR_WIDTH };
		let content_left = sidebar_width + CONTENT_MARGIN;
		let content_right = ( logical_width - CONTENT_MARGIN - 14 ).max( content_left + 280 );
		let content_width = content_right - content_left;
		let stacked = content_width < 620;
		let row_height = if stacked { GAP_BOX_HEIGHT } else { GAP_HEIGHT };
		let viewport_height = ( logical_height - HEADER_HEIGHT - FOOTER_HEIGHT ).max( 1 );
		let body_height = SECTION_HEIGHT + row_height * 3 + ROW_GAP * 2 + SECTION_GAP + SECTION_HEIGHT + row_height * 2 + ROW_GAP + 28;
		let scroll_max = ( body_height - viewport_height ).max( 0 );
		let scroll_y = scroll_y.clamp( 0, scroll_max );
		let mut y = HEADER_HEIGHT - scroll_y;
		let menu_section_title = logical_rect( content_left, y, content_right, y + SECTION_HEIGHT, dpi );
		y += SECTION_HEIGHT;
		let mut rows = Vec::with_capacity( 5 );
		for ( id, kind ) in [ ( SettingId::Overlay, ControlKind::Slider ), ( SettingId::Blur, ControlKind::Slider ), ( SettingId::AnimationDuration, ControlKind::Slider ) ] {
			rows.push( row_layout( id, kind, content_left, content_right, y, row_height, stacked, dpi ) );
			y += row_height + ROW_GAP;
		}
		y += SECTION_GAP - ROW_GAP;
		let tile_section_title = logical_rect( content_left, y, content_right, y + SECTION_HEIGHT, dpi );
		y += SECTION_HEIGHT;
		for ( id, kind ) in [ ( SettingId::BarColumns, ControlKind::Choice ), ( SettingId::TilesPerRow, ControlKind::Choice ) ] {
			rows.push( row_layout( id, kind, content_left, content_right, y, row_height, stacked, dpi ) );
			y += row_height + ROW_GAP;
		}
		let save_button = logical_rect( logical_width - 128, logical_height - 54, logical_width - 24, logical_height - 14, dpi );
		let undo_button = logical_rect( logical_width - 242, logical_height - 54, logical_width - 138, logical_height - 14, dpi );
		let scrollbar = if scroll_max > 0 {
			let track_top = HEADER_HEIGHT + 8;
			let track_bottom = ( logical_height - FOOTER_HEIGHT - 8 ).max( track_top + 1 );
			let track_height = track_bottom - track_top;
			let thumb_height = ( track_height * viewport_height / body_height.max( 1 ) ).clamp( SCROLLBAR_MIN_THUMB, track_height );
			let travel = track_height - thumb_height;
			let thumb_top = track_top + if scroll_max == 0 { 0 } else { travel * scroll_y / scroll_max };
			Some( ScrollbarLayout {
				track: logical_rect( logical_width - SCROLLBAR_MARGIN - SCROLLBAR_WIDTH, track_top, logical_width - SCROLLBAR_MARGIN, track_bottom, dpi ),
				thumb: logical_rect( logical_width - SCROLLBAR_MARGIN - SCROLLBAR_WIDTH, thumb_top, logical_width - SCROLLBAR_MARGIN, thumb_top + thumb_height, dpi ),
			} )
		} else { None };
		Self {
			sidebar: logical_rect( 0, 0, sidebar_width, logical_height, dpi ),
			sidebar_nav: logical_rect( 14, 88, sidebar_width - 14, 140, dpi ),
			settings_title: logical_rect( 28, 20, sidebar_width - 14, 66, dpi ),
			page_title: logical_rect( content_left, 16, content_right, 66, dpi ),
			menu_section_title,
			tile_section_title,
			undo_button,
			save_button,
			rows,
			scrollbar,
			expanded_sidebar,
			viewport_top: scale( HEADER_HEIGHT, dpi ),
			viewport_bottom: scale( logical_height - FOOTER_HEIGHT, dpi ),
			scroll_max,
		}
	}
	pub(crate) fn row( &self, id: SettingId ) -> Option< &SettingRowLayout > {
		self.rows.iter().find( |row| row.id == id )
	}
	pub(crate) fn hit_control( &self, x: i32, y: i32 ) -> Option< SettingId > {
		self.rows.iter().find( |row| contains( row.control, x, y ) ).map( |row| row.id )
	}
	pub(crate) fn hit_undo( &self, x: i32, y: i32 ) -> bool {
		contains( self.undo_button, x, y )
	}
	pub(crate) fn hit_save( &self, x: i32, y: i32 ) -> bool {
		contains( self.save_button, x, y )
	}
	pub(crate) fn hit_scroll_thumb( &self, x: i32, y: i32 ) -> bool {
		self.scrollbar.is_some_and( |scrollbar| contains( scrollbar.thumb, x, y ) )
	}
	pub(crate) fn hit_scroll_track( &self, x: i32, y: i32 ) -> bool {
		self.scrollbar.is_some_and( |scrollbar| contains( scrollbar.track, x, y ) )
	}
	pub(crate) fn scroll_from_thumb( &self, thumb_top: i32 ) -> i32 {
		let Some( scrollbar ) = self.scrollbar else { return 0; };
		let travel = ( scrollbar.track.bottom - scrollbar.track.top ) - ( scrollbar.thumb.bottom - scrollbar.thumb.top );
		if travel <= 0 { return 0; }
		( ( thumb_top - scrollbar.track.top ).clamp( 0, travel ) * self.scroll_max / travel ).clamp( 0, self.scroll_max )
	}
}

fn row_layout( id: SettingId, kind: ControlKind, left: i32, right: i32, top: i32, height: i32, stacked: bool, dpi: i32 ) -> SettingRowLayout {
	let card = logical_rect( left, top, right, top + height, dpi );
	let center_y = top + height / 2;
	let icon_size = 46;
	let icon = logical_rect( left + 18, center_y - icon_size / 2, left + 64, center_y + icon_size / 2, dpi );
	let text_left = left + 74;
	let ( title, description, control ) = if stacked {
		let title_top = top + 12;
		let description_top = title_top + 29;
		let control_height = 38;
		let control_bottom = top + height - 14;
		(
			logical_rect( text_left, title_top, right - 22, title_top + 31, dpi ),
			logical_rect( text_left, description_top, right - 22, description_top + 31, dpi ),
			logical_rect( text_left, control_bottom - control_height, right - 22, control_bottom, dpi ),
		)
	} else {
		let text_right = ( right - 324 ).max( text_left + 120 );
		let control_left = ( right - 300 ).max( text_right + 18 );
		let title_height = 30;
		let description_height = 28;
		let text_total_height = title_height + description_height;
		let text_top = center_y - text_total_height / 2;
		let control_height = 44;
		(
			logical_rect( text_left, text_top, text_right, text_top + title_height, dpi ),
			logical_rect( text_left, text_top + title_height - 2, text_right, text_top + title_height - 2 + description_height, dpi ),
			logical_rect( control_left, center_y - control_height / 2, right - 22, center_y + control_height / 2, dpi ),
		)
	};
	SettingRowLayout { id, kind, card, icon, title, description, control }
}

fn contains( rect: RECT, x: i32, y: i32 ) -> bool {
	x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
}

fn logical_rect( left: i32, top: i32, right: i32, bottom: i32, dpi: i32 ) -> RECT {
	RECT { left: scale( left, dpi ), top: scale( top, dpi ), right: scale( right, dpi ), bottom: scale( bottom, dpi ) }
}

pub(crate) fn scale( value: i32, dpi: i32 ) -> i32 {
	value * dpi / 96
}
pub(crate) fn unscale( value: i32, dpi: i32 ) -> i32 {
	value * 96 / dpi.max( 1 )
}