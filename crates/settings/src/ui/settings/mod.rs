//! ::  Project Path  ->  ep_start :: mod.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 16:45 周日


mod action;
mod item;
mod layout;
mod section;


pub( crate ) use action::{ ActionDefinition, ActionId };
pub( crate ) use item::{ ControlKind, SettingId, SettingItem, SettingItemLayout };
pub( crate ) use layout::{ SettingsUiLayout, SidebarItemLayout };
pub( crate ) use section::{ SectionId, SettingSection, SettingSubSection };


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub( crate ) enum SidebarItemId {
	Start,
}

#[derive( Clone, Copy )]
pub( crate ) struct SidebarItem {
	pub( crate ) id: SidebarItemId,
	pub( crate ) icon: &'static str,
	pub( crate ) selected: bool,
}

pub( crate ) struct SidebarDefinition {
	pub( crate ) main: Vec< SidebarItem >,
	pub( crate ) bottom: Vec< SidebarItem >,
}

#[allow( dead_code )]
pub( crate ) struct SidebarGroup< 'a > {
	items: &'a mut Vec< SidebarItem >,
}

pub( crate ) struct ContentDefinition {
	pub( crate ) sections: Vec< SettingSection >,
	pub( crate ) sub_sections: Vec< SettingSubSection >,
}

pub( crate ) struct FooterDefinition {
	pub( crate ) actions: Vec< ActionDefinition >,
}

pub( crate ) struct SettingsUi {
	pub( crate ) sidebar: SidebarDefinition,
	pub( crate ) content: ContentDefinition,
	pub( crate ) footer: FooterDefinition,
}

impl SidebarItem {
	pub( crate ) const fn new( id: SidebarItemId, icon: &'static str ) -> Self {
		Self { id, icon, selected: false }
	}
	pub( crate ) const fn selected( mut self ) -> Self {
		self.selected = true;
		self
	}
}

impl SidebarDefinition {
	pub( crate ) fn item( &mut self, item: SidebarItem ) {
		self.main.push( item );
	}
	pub( crate ) fn bottom( &mut self, configure: impl FnOnce( &mut SidebarGroup< '_ > ) ) {
		configure( &mut SidebarGroup { items: &mut self.bottom } );
	}
}

#[allow( dead_code )]
impl SidebarGroup< '_ > {
	pub( crate ) fn item( &mut self, item: SidebarItem ) {
		self.items.push( item );
	}
}

impl ContentDefinition {
	pub( crate ) fn section( &mut self, section: SettingSection ) {
		self.sections.push( section );
	}
	#[allow( dead_code )]
	pub( crate ) fn sub_section( &mut self, section: SettingSubSection ) {
		self.sub_sections.push( section );
	}
}

impl FooterDefinition {
	pub( crate ) fn action( &mut self, action: ActionDefinition ) {
		self.actions.push( action );
	}
}

impl SettingsUi {
	pub( crate ) fn new() -> Self {
		Self {
			sidebar: SidebarDefinition { main: Vec::new(), bottom: Vec::new() },
			content: ContentDefinition { sections: Vec::new(), sub_sections: Vec::new() },
			footer: FooterDefinition { actions: Vec::new() },
		}
	}
	pub( crate ) fn sidebar( mut self, configure: impl FnOnce( &mut SidebarDefinition ) ) -> Self {
		configure( &mut self.sidebar );
		self
	}
	pub( crate ) fn content( mut self, configure: impl FnOnce( &mut ContentDefinition ) ) -> Self {
		configure( &mut self.content );
		self
	}
	pub( crate ) fn footer( mut self, configure: impl FnOnce( &mut FooterDefinition ) ) -> Self {
		configure( &mut self.footer );
		self
	}
	pub( crate ) fn settings_page() -> Self {
		Self::new().sidebar( |sidebar| {
			sidebar.item( SidebarItem::new( SidebarItemId::Start, "🏠" ).selected() );
			sidebar.bottom( |_| {} );
		} ).content( |content| {
			content.section( SettingSection::new( SectionId::Behavior )
				.item( SettingItem::new( SettingId::Shortcut, ControlKind::Choice ) )
				.item( SettingItem::new( SettingId::StartButtonClick, ControlKind::Switch ) ) );
			content.section( SettingSection::new( SectionId::MenuBackground )
				.item( SettingItem::new( SettingId::Overlay, ControlKind::Slider ) )
				.item( SettingItem::new( SettingId::Blur, ControlKind::Slider ) )
				.item( SettingItem::new( SettingId::AnimationDuration, ControlKind::Slider ) ) );
			content.section( SettingSection::new( SectionId::Tiles )
				.item( SettingItem::new( SettingId::BarColumns, ControlKind::Choice ) )
				.item( SettingItem::new( SettingId::TilesPerRow, ControlKind::Choice ) )
				.item( SettingItem::new( SettingId::RoundedTiles, ControlKind::Switch ) )
				.item( SettingItem::new( SettingId::RoundedTileBars, ControlKind::Switch ) )
				.item( SettingItem::new( SettingId::TileAnimationDuration, ControlKind::Slider ) )
				.item( SettingItem::new( SettingId::TileBackgroundOpacity, ControlKind::Slider ) )
				.item( SettingItem::new( SettingId::TileBarBackgroundOpacity, ControlKind::Slider ) ) );
			content.section( SettingSection::new( SectionId::Debug )
				.item( SettingItem::new( SettingId::RestartShell, ControlKind::Button ) ) );
		} ).footer( |footer| {
			footer.action( ActionDefinition::new( ActionId::Undo, false ) );
			footer.action( ActionDefinition::new( ActionId::Save, true ) );
		} )
	}
	pub( crate ) fn layout( &self, client: windows::Win32::Foundation::RECT, dpi: i32, scroll_y: i32 ) -> SettingsUiLayout {
		SettingsUiLayout::calculate( self, client, dpi, scroll_y )
	}
}
