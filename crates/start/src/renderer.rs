//! ::  Project Path  ->  ep_start :: renderer.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 02:20 周六


use crate::animation::AnimationFrame;
use crate::config::{ StartConfig, Tile };
use crate::layout::{ DragSource, DragVisual, FolderTileAddress, TileAddress, TileLayout };
use windows_numerics::Matrix3x2;
use windows::Win32::Foundation::{ COLORREF, HWND, POINT, RECT, SIZE };
use windows::Win32::Graphics::Direct2D::Common::{ D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT };
use windows::Win32::Graphics::Direct2D::{ D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory, ID2D1SolidColorBrush };
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{ AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC, HGDIOBJ, SelectObject };
use windows::Win32::Graphics::DirectWrite::{ DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_TEXT_ALIGNMENT_LEADING, DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat };
use windows::Win32::UI::WindowsAndMessaging::{ GetWindowRect, ULW_ALPHA, UpdateLayeredWindow };
use windows::core::{ Result, w };


const TITLE_INSET: f32 = 10.0;
const PLACEHOLDER_SIZE: f32 = 52.0;


struct DeviceResources {
	target: ID2D1DCRenderTarget,
	surface: DibSurface,
	tile_brush: ID2D1SolidColorBrush,
	text_brush: ID2D1SolidColorBrush,
	hover_brush: ID2D1SolidColorBrush,
}


struct DibSurface {
	dc: HDC,
	bitmap: HBITMAP,
	previous: HGDIOBJ,
	size: D2D_SIZE_U,
}


pub struct Renderer {
	d2d_factory: ID2D1Factory,
	group_text_format: IDWriteTextFormat,
	tile_text_format: IDWriteTextFormat,
	icon_text_format: IDWriteTextFormat,
	device: Option< DeviceResources >,
}


impl Renderer {
	pub fn new() -> Result< Self > {
		unsafe {
			let d2d_factory = D2D1CreateFactory::< ID2D1Factory >( D2D1_FACTORY_TYPE_SINGLE_THREADED, None )?;
			let write_factory = DWriteCreateFactory::< IDWriteFactory >( DWRITE_FACTORY_TYPE_SHARED )?;
			let group_text_format = Self::create_text_format( &write_factory, 20.0 )?;
			let tile_text_format = Self::create_text_format( &write_factory, 14.0 )?;
			let icon_text_format = Self::create_text_format( &write_factory, 32.0 )?;
			Ok( Self { d2d_factory, group_text_format, tile_text_format, icon_text_format, device: None } )
		}
	}


	pub fn paint( &mut self, hwnd: HWND, size: D2D_SIZE_U, dpi: f32, config: &StartConfig, layout: &TileLayout, hovered: Option< TileAddress >, hovered_folder: Option< FolderTileAddress >, drag: Option< DragVisual >, drag_source_config: Option< &StartConfig >, drag_source_layout: Option< &TileLayout >, frame: &AnimationFrame ) -> Result< () > {
		self.ensure_device_resources( hwnd, size, dpi )?;
		let device = self.device.as_ref().expect( "渲染资源应已创建" );
		unsafe {
			let bounds = RECT { left: 0, top: 0, right: size.width as i32, bottom: size.height as i32 };
			device.target.BindDC( device.surface.dc, &bounds )?;
			device.target.BeginDraw();
			device.target.Clear( Some( &D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 } ) );
			for bar_region in &layout.bars {
				if matches!( drag.as_ref().map( |value| value.preview_source ), Some( DragSource::Bar( bar_index ) ) if bar_index == bar_region.bar_index ) { continue; }
				let progress = frame.group_progress( bar_region.bar_index );
				if progress <= 0.0 { continue; }
				let bar = &config.bars[ bar_region.bar_index ];
				device.text_brush.SetOpacity( progress );
				device.target.SetTransform( &animation_transform( &bar_region.title_rect, progress, 18.0 ) );
				Self::draw_text( &device.target, &device.text_brush, &self.group_text_format, &bar.title, &bar_region.title_rect );
			}
			for ( render_index, tile_region ) in layout.tiles.iter().enumerate() {
				if matches!( drag.as_ref().map( |value| value.preview_source ), Some( DragSource::Tile( address ) ) if address == tile_region.address ) || matches!( drag.as_ref().map( |value| value.preview_source ), Some( DragSource::Bar( bar_index ) ) if bar_index == tile_region.address.bar_index ) { continue; }
				let progress = frame.tile_progress( render_index );
				let opacity = frame.tile_opacity( render_index );
				if opacity <= 0.0 { continue; }
				let tile = &config.bars[ tile_region.address.bar_index ].tiles[ tile_region.address.tile_index ];
				device.tile_brush.SetColor( &parse_color( &tile.color ) );
				device.tile_brush.SetOpacity( opacity );
				device.text_brush.SetOpacity( opacity );
				device.target.SetTransform( &animation_transform( &tile_region.rect, progress, 28.0 ) );
				device.target.FillRectangle( &tile_region.rect, &device.tile_brush );
				if hovered == Some( tile_region.address ) { device.hover_brush.SetOpacity( 0.12 * opacity ); device.target.FillRectangle( &tile_region.rect, &device.hover_brush ); device.target.DrawRectangle( &tile_region.rect, &device.hover_brush, 1.0, None ); }
				self.draw_tile_icon( device, tile, &tile_region.rect );
				let title_rect = D2D_RECT_F { left: tile_region.rect.left + TITLE_INSET, top: tile_region.rect.bottom - 31.0, right: tile_region.rect.right - TITLE_INSET, bottom: tile_region.rect.bottom - 8.0 };
				Self::draw_text( &device.target, &device.text_brush, &self.tile_text_format, &tile.title, &title_rect );
			}
			self.draw_folder_panel( device, config, layout, hovered_folder );
			if let Some( drag ) = drag { self.draw_drag_visual( device, drag_source_config.unwrap_or( config ), drag_source_layout.unwrap_or( layout ), layout, &drag ); }
			device.target.SetTransform( &identity_transform() );
			if let Err( error ) = device.target.EndDraw( None, None ) {
				self.device = None;
				return Err( error );
			}
			device.surface.present( hwnd )?;
		}
		Ok( () )
	}


	fn draw_tile_icon( &self, device: &DeviceResources, tile: &Tile, rect: &D2D_RECT_F ) {
		let center_x = ( rect.left + rect.right ) * 0.5;
		let icon_rect = D2D_RECT_F { left: center_x - PLACEHOLDER_SIZE * 0.5, top: rect.top + 30.0, right: center_x + PLACEHOLDER_SIZE * 0.5, bottom: rect.top + 30.0 + PLACEHOLDER_SIZE };
		if tile.is_folder() {
			let size = 23.0;
			let gap = 4.0;
			for ( index, child ) in tile.tiles.iter().take( 4 ).enumerate() {
				let column = index % 2;
				let row = index / 2;
				let left = icon_rect.left + column as f32 * ( size + gap );
				let top = icon_rect.top + row as f32 * ( size + gap );
				unsafe { device.tile_brush.SetColor( &parse_color( &child.color ) ); device.tile_brush.SetOpacity( 1.0 ); device.target.FillRectangle( &D2D_RECT_F { left, top, right: left + size, bottom: top + size }, &device.tile_brush ); }
			}
		} else {
			let icon_text = tile.title.chars().next().unwrap_or( ' ' ).to_string();
			Self::draw_text( &device.target, &device.text_brush, &self.icon_text_format, &icon_text, &D2D_RECT_F { left: icon_rect.left + 15.0, top: icon_rect.top + 5.0, right: icon_rect.right, bottom: icon_rect.bottom } );
		}
	}


	fn draw_drag_visual( &self, device: &DeviceResources, source_config: &StartConfig, source_layout: &TileLayout, preview_layout: &TileLayout, drag: &DragVisual ) {
		unsafe {
			device.target.SetTransform( &translation_transform( drag.delta_x, drag.delta_y ) );
			match drag.source {
				DragSource::Tile( address ) => {
					{
						let rect = drag.origin_rect;
						let tile = &source_config.bars[ address.bar_index ].tiles[ address.tile_index ];
						device.tile_brush.SetColor( &parse_color( &tile.color ) );
						device.tile_brush.SetOpacity( 0.82 );
						device.text_brush.SetOpacity( 0.9 );
						device.target.FillRectangle( &rect, &device.tile_brush );
						let title_rect = D2D_RECT_F { left: rect.left + TITLE_INSET, top: rect.bottom - 31.0, right: rect.right - TITLE_INSET, bottom: rect.bottom - 8.0 };
						Self::draw_text( &device.target, &device.text_brush, &self.tile_text_format, &tile.title, &title_rect );
					}
				}
				DragSource::Bar( bar_index ) => {
					if let Some( bar_region ) = source_layout.bars.iter().find( |bar| bar.bar_index == bar_index ) {
						device.text_brush.SetOpacity( 0.9 );
						Self::draw_text( &device.target, &device.text_brush, &self.group_text_format, &source_config.bars[ bar_index ].title, &bar_region.title_rect );
						for tile_region in source_layout.tiles.iter().filter( |tile| tile.address.bar_index == bar_index ) {
							let tile = &source_config.bars[ bar_index ].tiles[ tile_region.address.tile_index ];
							device.tile_brush.SetColor( &parse_color( &tile.color ) );
							device.tile_brush.SetOpacity( 0.72 );
							device.target.FillRectangle( &tile_region.rect, &device.tile_brush );
						}
					}
				}
			}
			device.target.SetTransform( &identity_transform() );
			if let Some( target_rect ) = preview_layout.drop_rect( drag.target ) { device.hover_brush.SetOpacity( 0.9 ); device.target.DrawRectangle( &target_rect, &device.hover_brush, 3.0, None ); }
		}
	}


	fn draw_folder_panel( &self, device: &DeviceResources, config: &StartConfig, layout: &TileLayout, hovered: Option< FolderTileAddress > ) {
		let Some( panel ) = &layout.folder_panel else { return; };
		unsafe {
			device.target.SetTransform( &identity_transform() );
			device.tile_brush.SetColor( &D2D1_COLOR_F { r: 0.08, g: 0.08, b: 0.09, a: 0.97 } );
			device.tile_brush.SetOpacity( 0.97 );
			device.target.FillRectangle( &panel.rect, &device.tile_brush );
			let folder = &config.bars[ panel.owner.bar_index ].tiles[ panel.owner.tile_index ];
			for tile_region in &panel.tiles {
				let tile = &folder.tiles[ tile_region.address.tile_index ];
				device.tile_brush.SetColor( &parse_color( &tile.color ) );
				device.tile_brush.SetOpacity( 1.0 );
				device.text_brush.SetOpacity( 1.0 );
				device.target.FillRectangle( &tile_region.rect, &device.tile_brush );
				if hovered == Some( tile_region.address ) { device.hover_brush.SetOpacity( 0.14 ); device.target.FillRectangle( &tile_region.rect, &device.hover_brush ); }
				let title_rect = D2D_RECT_F { left: tile_region.rect.left + TITLE_INSET, top: tile_region.rect.bottom - 31.0, right: tile_region.rect.right - TITLE_INSET, bottom: tile_region.rect.bottom - 8.0 };
				Self::draw_text( &device.target, &device.text_brush, &self.tile_text_format, &tile.title, &title_rect );
			}
		}
	}


	pub fn prepare( &mut self, hwnd: HWND, size: D2D_SIZE_U, dpi: f32 ) -> Result< () > {
		self.ensure_device_resources( hwnd, size, dpi )
	}


	pub fn resize( &mut self, size: D2D_SIZE_U, dpi: f32 ) {
		if self.device.as_ref().is_some_and( |device| device.surface.size != size ) { self.device = None; }
		if let Some( device ) = &self.device { unsafe { device.target.SetDpi( dpi, dpi ); } }
	}


	pub fn release_device_resources( &mut self ) {
		self.device = None;
	}


	fn ensure_device_resources( &mut self, _hwnd: HWND, size: D2D_SIZE_U, dpi: f32 ) -> Result< () > {
		if self.device.as_ref().is_some_and( |device| device.surface.size == size ) { return Ok( () ); }
		self.device = None;
		unsafe {
			let properties = D2D1_RENDER_TARGET_PROPERTIES { r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT, pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED }, dpiX: dpi, dpiY: dpi, usage: D2D1_RENDER_TARGET_USAGE_NONE, minLevel: D2D1_FEATURE_LEVEL_DEFAULT };
			let target = self.d2d_factory.CreateDCRenderTarget( &properties )?;
			target.SetTextAntialiasMode( D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE );
			let surface = DibSurface::create( size )?;
			target.SetDpi( dpi, dpi );
			let tile_brush = target.CreateSolidColorBrush( &parse_color( "#0067C0" ), None )?;
			let text_brush = target.CreateSolidColorBrush( &D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, None )?;
			let hover_brush = target.CreateSolidColorBrush( &D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, None )?;
			self.device = Some( DeviceResources { target, surface, tile_brush, text_brush, hover_brush } );
		}
		Ok( () )
	}


	fn create_text_format( factory: &IDWriteFactory, size: f32 ) -> Result< IDWriteTextFormat > {
		unsafe {
			let format = factory.CreateTextFormat( w!( "Microsoft YaHei UI" ), None, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, size, w!( "zh-CN" ) )?;
			format.SetTextAlignment( DWRITE_TEXT_ALIGNMENT_LEADING )?;
			format.SetParagraphAlignment( DWRITE_PARAGRAPH_ALIGNMENT_NEAR )?;
			Ok( format )
		}
	}


	fn draw_text( target: &ID2D1DCRenderTarget, brush: &ID2D1SolidColorBrush, format: &IDWriteTextFormat, text: &str, rect: &D2D_RECT_F ) {
		let utf16: Vec< u16 > = text.encode_utf16().collect();
		unsafe { target.DrawText( &utf16, format, rect, brush, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL ); }
	}
}


impl DibSurface {
	fn create( size: D2D_SIZE_U ) -> Result< Self > {
		let info = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::< BITMAPINFOHEADER >() as u32, biWidth: size.width as i32, biHeight: -( size.height as i32 ), biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() }, ..Default::default() };
		let mut bits = std::ptr::null_mut();
		unsafe {
			let dc = CreateCompatibleDC( None );
			if dc.is_invalid() { return Err( windows::core::Error::from_thread() ); }
			let bitmap = match CreateDIBSection( None, &info, DIB_RGB_COLORS, &mut bits, None, 0 ) { Ok( bitmap ) => bitmap, Err( error ) => { let _ = DeleteDC( dc ); return Err( error ); } };
			let previous = SelectObject( dc, HGDIOBJ( bitmap.0 ) );
			Ok( Self { dc, bitmap, previous, size } )
		}
	}


	fn present( &self, hwnd: HWND ) -> Result< () > {
		let mut window_rect = RECT::default();
		unsafe { GetWindowRect( hwnd, &mut window_rect )?; }
		let destination = POINT { x: window_rect.left, y: window_rect.top };
		let source = POINT::default();
		let size = SIZE { cx: self.size.width as i32, cy: self.size.height as i32 };
		let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
		unsafe { UpdateLayeredWindow( hwnd, None, Some( &destination ), Some( &size ), Some( self.dc ), Some( &source ), COLORREF( 0 ), Some( &blend ), ULW_ALPHA ) }
	}
}


impl Drop for DibSurface {
	fn drop( &mut self ) {
		unsafe { SelectObject( self.dc, self.previous ); let _ = DeleteObject( HGDIOBJ( self.bitmap.0 ) ); let _ = DeleteDC( self.dc ); }
	}
}


fn parse_color( value: &str ) -> D2D1_COLOR_F {
	let value = value.strip_prefix( "#" ).unwrap_or( value );
	if value.len() == 6 {
		if let Ok( color ) = u32::from_str_radix( value, 16 ) {
			return D2D1_COLOR_F { r: ( ( color >> 16 ) & 0xFF ) as f32 / 255.0, g: ( ( color >> 8 ) & 0xFF ) as f32 / 255.0, b: ( color & 0xFF ) as f32 / 255.0, a: 1.0 };
		}
	}
	D2D1_COLOR_F { r: 0.0, g: 0.404, b: 0.753, a: 1.0 }
}


fn animation_transform( rect: &D2D_RECT_F, progress: f32, offset: f32 ) -> Matrix3x2 {
	let scale = 0.985 + 0.015 * progress;
	let center_x = ( rect.left + rect.right ) * 0.5;
	let center_y = ( rect.top + rect.bottom ) * 0.5;
	Matrix3x2 { M11: scale, M12: 0.0, M21: 0.0, M22: scale, M31: center_x * ( 1.0 - scale ), M32: center_y * ( 1.0 - scale ) + offset * ( 1.0 - progress ) }
}


fn identity_transform() -> Matrix3x2 {
	Matrix3x2 { M11: 1.0, M12: 0.0, M21: 0.0, M22: 1.0, M31: 0.0, M32: 0.0 }
}


fn translation_transform( x: f32, y: f32 ) -> Matrix3x2 {
	Matrix3x2 { M11: 1.0, M12: 0.0, M21: 0.0, M22: 1.0, M31: x, M32: y }
}
