//! ::  Project Path  ->  ep_start :: geometry.rs :: geometry
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 02:52 周日


use windows::Win32::Foundation::RECT;


#[derive( Clone, Copy, Debug, Default, Eq, PartialEq )]
pub( crate ) struct UiRect {
	pub left: i32,
	pub top: i32,
	pub right: i32,
	pub bottom: i32,
}


impl UiRect {
	pub const fn new( left: i32, top: i32, right: i32, bottom: i32 ) -> Self {
		Self { left, top, right, bottom }
	}
	pub const fn from_size( left: i32, top: i32, width: i32, height: i32 ) -> Self {
		Self {
			left,
			top,
			right: left + width,
			bottom: top + height,
		}
	}

	pub const fn width( &self ) -> i32 {
		self.right - self.left
	}
	pub const fn height( &self ) -> i32 {
		self.bottom - self.top
	}
	pub const fn center_x( &self ) -> i32 {
		( self.left + self.right ) / 2
	}
	pub const fn center_y( &self ) -> i32 {
		( self.top + self.bottom ) / 2
	}
	pub const fn inset( &self, x: i32, y: i32 ) -> Self {
		Self {
			left: self.left + x,
			top: self.top + y,
			right: self.right - x,
			bottom: self.bottom - y,
		}
	}
	pub const fn offset( &self, x: i32, y: i32 ) -> Self {
		Self {
			left: self.left + x,
			top: self.top + y,
			right: self.right + x,
			bottom: self.bottom + y,
		}
	}
	pub const fn contains( &self, x: i32, y: i32 ) -> bool {
		x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
	}
	pub const fn to_rect( &self ) -> RECT {
		RECT {
			left: self.left,
			top: self.top,
			right: self.right,
			bottom: self.bottom,
		}
	}
}

impl From< RECT > for UiRect {
	fn from( value: RECT ) -> Self {
		Self {
			left: value.left,
			top: value.top,
			right: value.right,
			bottom: value.bottom,
		}
	}
}

impl From< UiRect > for RECT {
	fn from( value: UiRect ) -> Self {
		value.to_rect()
	}
}

pub( crate ) const fn scale( value: i32, dpi: i32 ) -> i32 {
	value * dpi / 96
}
pub( crate ) const fn unscale( value: i32, dpi: i32 ) -> i32 {
	value * 96 / if dpi <= 0 { 1 } else { dpi }
}
pub( crate ) const fn logical_rect( left: i32, top: i32, right: i32, bottom: i32, dpi: i32 ) -> UiRect {
	UiRect {
		left: scale( left, dpi ),
		top: scale( top, dpi ),
		right: scale( right, dpi ),
		bottom: scale( bottom, dpi ),
	}
}