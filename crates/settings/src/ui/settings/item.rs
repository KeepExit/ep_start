//! ::  Project Path  ->  ep_start :: item.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 16:45 周日


use windows::Win32::Foundation::RECT;


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub( crate ) enum SettingId {
	Overlay,
	Blur,
	AnimationDuration,
	Shortcut,
	StartButtonClick,
	BarColumns,
	TilesPerRow,
	RestartShell,
}

#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub( crate ) enum ControlKind {
	Slider,
	Choice,
	Switch,
	Button,
}

#[derive( Clone, Copy )]
pub( crate ) struct SettingItem {
	pub( crate ) id: SettingId,
	pub( crate ) control: ControlKind,
}

#[derive( Clone, Copy )]
pub( crate ) struct SettingItemLayout {
	pub( crate ) id: SettingId,
	pub( crate ) control_kind: ControlKind,
	pub( crate ) card: RECT,
	pub( crate ) icon: RECT,
	pub( crate ) title: RECT,
	pub( crate ) description: RECT,
	pub( crate ) control: RECT,
}

impl SettingItem {
	pub( crate ) const fn new( id: SettingId, control: ControlKind ) -> Self {
		Self { id, control }
	}
}
