//! ::  Project Path  ->  ep_start :: section.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 16:45 周日


use super::item::{ SettingId, SettingItem, SettingItemLayout };
use windows::Win32::Foundation::RECT;


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub( crate ) enum SectionId {
	Behavior,
	MenuBackground,
	Tiles,
}

pub( crate ) struct SettingSection {
	pub( crate ) id: SectionId,
	pub( crate ) items: Vec< SettingItem >,
}

#[allow( dead_code )]
pub( crate ) struct SettingSubSection {
	pub( crate ) parent: SettingId,
	pub( crate ) items: Vec< SettingItem >,
}

pub( crate ) struct SectionLayout {
	pub( crate ) id: SectionId,
	#[allow( dead_code )]
	pub( crate ) bounds: RECT,
	pub( crate ) title: RECT,
	pub( crate ) items: Vec< SettingItemLayout >,
}

#[allow( dead_code )]
pub( crate ) struct SubSectionLayout {
	pub( crate ) parent: SettingId,
	pub( crate ) bounds: RECT,
	pub( crate ) items: Vec< SettingItemLayout >,
}

impl SettingSection {
	pub( crate ) fn new( id: SectionId ) -> Self {
		Self { id, items: Vec::new() }
	}
	pub( crate ) fn item( mut self, item: SettingItem ) -> Self {
		self.items.push( item );
		self
	}
}

#[allow( dead_code )]
impl SettingSubSection {
	pub( crate ) fn new( parent: SettingId ) -> Self {
		Self { parent, items: Vec::new() }
	}
	pub( crate ) fn item( mut self, item: SettingItem ) -> Self {
		self.items.push( item );
		self
	}
}
