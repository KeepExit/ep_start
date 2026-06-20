//! ::  Project Path  ->  ep_start :: paint_buffer.rs :: paint_buffer
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 03:38 周日


use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::{ BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, HDC, HGDIOBJ, SRCCOPY, SelectObject };


pub( crate ) fn paint_buffered( hdc: HDC, client: RECT, paint: impl FnOnce( HDC ) ) {
	let width = ( client.right - client.left ).max( 1 );
	let height = ( client.bottom - client.top ).max( 1 );
	unsafe {
		let buffer_dc = CreateCompatibleDC( Some( hdc ) );
		if buffer_dc.is_invalid() {
			paint( hdc );
			return;
		}
		let buffer_bitmap = CreateCompatibleBitmap( hdc, width, height );
		if buffer_bitmap.is_invalid() {
			let _ = DeleteDC( buffer_dc );
			paint( hdc );
			return;
		}
		let previous = SelectObject( buffer_dc, HGDIOBJ( buffer_bitmap.0 ) );
		paint( buffer_dc );
		let _ = BitBlt( hdc, 0, 0, width, height, Some( buffer_dc ), 0, 0, SRCCOPY );
		SelectObject( buffer_dc, previous );
		let _ = DeleteObject( HGDIOBJ( buffer_bitmap.0 ) );
		let _ = DeleteDC( buffer_dc );
	}
}