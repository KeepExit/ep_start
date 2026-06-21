//! ::  Project Path  ->  ep_start :: action.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 16:45 周日


use windows::Win32::Foundation::RECT;


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub( crate ) enum ActionId {
	Undo,
	Save,
}

#[derive( Clone, Copy )]
pub( crate ) struct ActionDefinition {
	pub( crate ) id: ActionId,
	pub( crate ) primary: bool,
}

#[derive( Clone, Copy )]
pub( crate ) struct ActionLayout {
	pub( crate ) id: ActionId,
	pub( crate ) area: RECT,
	pub( crate ) primary: bool,
}

impl ActionDefinition {
	pub( crate ) const fn new( id: ActionId, primary: bool ) -> Self {
		Self { id, primary }
	}
}
