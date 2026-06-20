//! ::  Project Path  ->  ep_start :: painter.rs :: painter
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 03:04 周日


use crate::ui::geometry::{ UiRect, scale };
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{ CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, COLORONCOLOR, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET, DEFAULT_PITCH, DRAW_TEXT_FORMAT, DT_CENTER, DT_LEFT, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, Ellipse, FF_DONTCARE, FillRect, GetStockObject, HALFTONE, HDC, HGDIOBJ, NULL_PEN, OUT_DEFAULT_PRECIS, RestoreDC, RoundRect, SRCCOPY, SaveDC, SelectObject, SetBkMode, SetStretchBltMode, SetTextColor, StretchBlt, TRANSPARENT };
use windows::core::w;


pub( crate ) struct Painter {
	hdc: HDC,
	dpi: i32,
}

impl Painter {
	pub( crate ) const fn new( hdc: HDC, dpi: i32 ) -> Self {
		Self { hdc, dpi }
	}

	pub( crate ) const fn dpi( &self ) -> i32 {
		self.dpi
	}
	pub( crate ) fn scale( &self, value: i32 ) -> i32 {
		scale( value, self.dpi )
	}
	pub( crate ) fn fill( &self, area: impl Into< UiRect >, color: COLORREF ) {
		let area = area.into();
		let brush = unsafe { CreateSolidBrush( color ) };
		unsafe {
			FillRect( self.hdc, &area.to_rect(), brush );
			let _ = DeleteObject( HGDIOBJ( brush.0 ) );
		}
	}
	pub( crate ) fn round_rect( &self, area: impl Into< UiRect >, radius: i32, color: COLORREF ) {
		let area = area.into();
		let brush = unsafe { CreateSolidBrush( color ) };
		let old_brush = unsafe { SelectObject( self.hdc, HGDIOBJ( brush.0 ) ) };
		let old_pen = unsafe { SelectObject( self.hdc, GetStockObject( NULL_PEN ) ) };
		let radius = self.scale( radius );
		unsafe {
			let _ = RoundRect( self.hdc, area.left, area.top, area.right, area.bottom, radius, radius );
			let _ = SelectObject( self.hdc, old_pen );
			let _ = SelectObject( self.hdc, old_brush );
			let _ = DeleteObject( HGDIOBJ( brush.0 ) );
		}
	}
	pub( crate ) fn text( &self, text: &str, area: impl Into< UiRect >, size: i32, weight: i32, color: COLORREF ) {
		self.draw_text( text, area.into(), size, weight, color, DT_LEFT | DT_SINGLELINE | DT_VCENTER );
	}
	pub( crate ) fn center_text( &self, text: &str, area: impl Into< UiRect >, size: i32, weight: i32, color: COLORREF ) {
		self.draw_text( text, area.into(), size, weight, color, DT_CENTER | DT_SINGLELINE | DT_VCENTER );
	}
	pub( crate ) fn right_text( &self, text: &str, area: impl Into< UiRect >, size: i32, weight: i32, color: COLORREF ) {
		self.draw_text( text, area.into(), size, weight, color, DT_RIGHT | DT_SINGLELINE | DT_VCENTER );
	}
	pub( crate ) fn antialiased_thumb( &self, x: i32, y: i32, outer_radius: i32, inner_radius: i32, outer_color: COLORREF, inner_color: COLORREF ) {
		const SUPERSAMPLE: i32 = 4;
		let outer_radius = self.scale( outer_radius );
		let inner_radius = self.scale( inner_radius );
		let padding = self.scale( 2 );
		let size = ( outer_radius + padding ) * 2;
		let left = x - size / 2;
		let top = y - size / 2;
		let high_size = size * SUPERSAMPLE;
		unsafe {
			let high_dc = CreateCompatibleDC( Some( self.hdc ) );
			if high_dc.is_invalid() {
				self.solid_circle( x, y, outer_radius, outer_color );
				self.solid_circle( x, y, inner_radius, inner_color );
				return;
			}
			let high_bitmap = CreateCompatibleBitmap( self.hdc, high_size, high_size );
			if high_bitmap.is_invalid() {
				let _ = DeleteDC( high_dc );
				self.solid_circle( x, y, outer_radius, outer_color );
				self.solid_circle( x, y, inner_radius, inner_color );
				return;
			}
			let previous = SelectObject( high_dc, HGDIOBJ( high_bitmap.0 ) );
			SetStretchBltMode( high_dc, COLORONCOLOR );
			let _ = StretchBlt( high_dc, 0, 0, high_size, high_size, Some( self.hdc ), left, top, size, size, SRCCOPY );
			let center = high_size / 2;
			solid_circle_raw( high_dc, center, center, outer_radius * SUPERSAMPLE, outer_color );
			solid_circle_raw( high_dc, center, center, inner_radius * SUPERSAMPLE, inner_color );
			let saved = SaveDC( self.hdc );
			SetStretchBltMode( self.hdc, HALFTONE );
			let _ = StretchBlt( self.hdc, left, top, size, size, Some( high_dc ), 0, 0, high_size, high_size, SRCCOPY );
			let _ = RestoreDC( self.hdc, saved );
			SelectObject( high_dc, previous );
			let _ = DeleteObject( HGDIOBJ( high_bitmap.0 ) );
			let _ = DeleteDC( high_dc );
		}
	}
	fn draw_text( &self, text: &str, area: UiRect, size: i32, weight: i32, color: COLORREF, format: DRAW_TEXT_FORMAT ) {
		let font = unsafe { CreateFontW( -self.scale( size ), 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, DEFAULT_PITCH.0 as u32 | FF_DONTCARE.0 as u32, w!( "Microsoft YaHei UI" ) ) };
		let previous = unsafe { SelectObject( self.hdc, HGDIOBJ( font.0 ) ) };
		let mut wide: Vec< u16 > = text.encode_utf16().collect();
		let mut area = area.to_rect();
		unsafe {
			SetBkMode( self.hdc, TRANSPARENT );
			SetTextColor( self.hdc, color );
			DrawTextW( self.hdc, &mut wide, &mut area, format );
			let _ = SelectObject( self.hdc, previous );
			let _ = DeleteObject( HGDIOBJ( font.0 ) );
		}
	}
	fn solid_circle( &self, x: i32, y: i32, radius: i32, color: COLORREF ) {
		solid_circle_raw( self.hdc, x, y, radius, color );
	}
}

fn solid_circle_raw( hdc: HDC, x: i32, y: i32, radius: i32, color: COLORREF ) {
	let brush = unsafe { CreateSolidBrush( color ) };
	let old_brush = unsafe { SelectObject( hdc, HGDIOBJ( brush.0 ) ) };
	let old_pen = unsafe { SelectObject( hdc, GetStockObject( NULL_PEN ) ) };
	unsafe {
		let _ = Ellipse( hdc, x - radius, y - radius, x + radius, y + radius );
		let _ = SelectObject( hdc, old_pen );
		let _ = SelectObject( hdc, old_brush );
		let _ = DeleteObject( HGDIOBJ( brush.0 ) );
	}
}