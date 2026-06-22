//! ::  Project Path  ->  ep_start :: renderer.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 02:20 周六


use crate::animation::AnimationFrame;
use crate::config::{ StartConfig, Tile };
use crate::layout::{ DragSource, DragVisual, FolderTileAddress, TileAddress, TileLayout };
use windows_numerics::Matrix3x2;
use windows::Win32::Foundation::{ HMODULE, HWND };
use windows::Win32::Graphics::Direct2D::Common::{ D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT };
use windows::Win32::Graphics::Direct2D::{ D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateDevice, ID2D1Bitmap1, ID2D1Device, ID2D1DeviceContext, ID2D1SolidColorBrush };
use windows::Win32::Graphics::Direct3D::{ D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL };
use windows::Win32::Graphics::Direct3D11::{ D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device };
use windows::Win32::Graphics::DirectComposition::{ DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual };
use windows::Win32::Graphics::DirectWrite::{ DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_TEXT_ALIGNMENT_LEADING, DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat };
use windows::Win32::Graphics::Dxgi::Common::{ DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC };
use windows::Win32::Graphics::Dxgi::{ DXGI_PRESENT, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIAdapter, IDXGIDevice, IDXGIDevice1, IDXGIFactory2, IDXGIOutput, IDXGISurface, IDXGISwapChain1 };
use windows::core::{ Interface, Result, w };


const TITLE_INSET: f32 = 10.0;
const PLACEHOLDER_SIZE: f32 = 52.0;


struct DeviceResources {
	target: ID2D1DeviceContext,
	swap_chain: IDXGISwapChain1,
	_target_bitmap: ID2D1Bitmap1,
	_d3d_device: ID3D11Device,
	_d2d_device: ID2D1Device,
	_composition_device: IDCompositionDevice,
	_composition_target: IDCompositionTarget,
	_composition_visual: IDCompositionVisual,
	tile_brush: ID2D1SolidColorBrush,
	text_brush: ID2D1SolidColorBrush,
	hover_brush: ID2D1SolidColorBrush,
	size: D2D_SIZE_U,
}


pub struct Renderer {
	group_text_format: IDWriteTextFormat,
	tile_text_format: IDWriteTextFormat,
	icon_text_format: IDWriteTextFormat,
	device: Option< DeviceResources >,
}


impl Renderer {
	pub fn new() -> Result< Self > {
		unsafe {
			let write_factory = DWriteCreateFactory::< IDWriteFactory >( DWRITE_FACTORY_TYPE_SHARED )?;
			let group_text_format = Self::create_text_format( &write_factory, 20.0 )?;
			let tile_text_format = Self::create_text_format( &write_factory, 14.0 )?;
			let icon_text_format = Self::create_text_format( &write_factory, 32.0 )?;
			Ok( Self { group_text_format, tile_text_format, icon_text_format, device: None } )
		}
	}


	pub fn paint( &mut self, hwnd: HWND, size: D2D_SIZE_U, dpi: f32, config: &StartConfig, layout: &TileLayout, hovered: Option< TileAddress >, hovered_folder: Option< FolderTileAddress >, renaming_bar: Option< usize >, drag: Option< DragVisual >, drag_source_config: Option< &StartConfig >, drag_source_layout: Option< &TileLayout >, frame: &AnimationFrame ) -> Result< () > {
		self.ensure_device_resources( hwnd, size, dpi )?;
		let device = self.device.as_ref().expect( "渲染资源应已创建" );
		unsafe {
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
				if renaming_bar == Some( bar_region.bar_index ) { device.hover_brush.SetOpacity( 0.72 * progress ); device.target.DrawRectangle( &bar_region.title_rect, &device.hover_brush, 1.0, None ); }
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
			device.swap_chain.Present( 1, DXGI_PRESENT( 0 ) ).ok()?;
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
		if self.device.as_ref().is_some_and( |device| device.size != size ) { self.device = None; }
		if let Some( device ) = &self.device { unsafe { device.target.SetDpi( dpi, dpi ); } }
	}


	pub fn release_device_resources( &mut self ) {
		self.device = None;
	}


	fn ensure_device_resources( &mut self, hwnd: HWND, size: D2D_SIZE_U, dpi: f32 ) -> Result< () > {
		if self.device.as_ref().is_some_and( |device| device.size == size ) { return Ok( () ); }
		self.device = None;
		unsafe {
			let d3d_device = Self::create_d3d_device( D3D_DRIVER_TYPE_HARDWARE ).or_else( |_| Self::create_d3d_device( D3D_DRIVER_TYPE_WARP ) )?;
			let dxgi_device: IDXGIDevice = d3d_device.cast()?;
			let dxgi_device1: IDXGIDevice1 = dxgi_device.cast()?;
			dxgi_device1.SetMaximumFrameLatency( 1 )?;
			let adapter = dxgi_device.GetAdapter()?;
			let factory: IDXGIFactory2 = adapter.GetParent()?;
			let description = DXGI_SWAP_CHAIN_DESC1 { Width: size.width, Height: size.height, Format: DXGI_FORMAT_B8G8R8A8_UNORM, Stereo: false.into(), SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 }, BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT, BufferCount: 2, Scaling: DXGI_SCALING_STRETCH, SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED, Flags: 0 };
			let swap_chain = factory.CreateSwapChainForComposition( &d3d_device, &description, None::< &IDXGIOutput > )?;
			let surface: IDXGISurface = swap_chain.GetBuffer( 0 )?;
			let d2d_device = D2D1CreateDevice( &dxgi_device, None )?;
			let target = d2d_device.CreateDeviceContext( D2D1_DEVICE_CONTEXT_OPTIONS_NONE )?;
			let bitmap_properties = D2D1_BITMAP_PROPERTIES1 { pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED }, dpiX: dpi, dpiY: dpi, bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW, ..Default::default() };
			let target_bitmap = target.CreateBitmapFromDxgiSurface( &surface, Some( &bitmap_properties ) )?;
			target.SetTarget( &target_bitmap );
			target.SetTextAntialiasMode( D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE );
			target.SetDpi( dpi, dpi );
			let tile_brush = target.CreateSolidColorBrush( &parse_color( "#0067C0" ), None )?;
			let text_brush = target.CreateSolidColorBrush( &D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, None )?;
			let hover_brush = target.CreateSolidColorBrush( &D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, None )?;
			let composition_device = DCompositionCreateDevice::< _, IDCompositionDevice >( &dxgi_device )?;
			let composition_target = composition_device.CreateTargetForHwnd( hwnd, true )?;
			let composition_visual = composition_device.CreateVisual()?;
			composition_visual.SetContent( &swap_chain )?;
			composition_target.SetRoot( &composition_visual )?;
			composition_device.Commit()?;
			self.device = Some( DeviceResources { target, swap_chain, _target_bitmap: target_bitmap, _d3d_device: d3d_device, _d2d_device: d2d_device, _composition_device: composition_device, _composition_target: composition_target, _composition_visual: composition_visual, tile_brush, text_brush, hover_brush, size } );
		}
		Ok( () )
	}


	fn create_d3d_device( driver_type: D3D_DRIVER_TYPE ) -> Result< ID3D11Device > {
		let mut device = None;
		let mut feature_level = D3D_FEATURE_LEVEL::default();
		unsafe { D3D11CreateDevice( None::< &IDXGIAdapter >, driver_type, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some( &mut device ), Some( &mut feature_level ), None )?; }
		device.ok_or_else( windows::core::Error::from_thread )
	}


	fn create_text_format( factory: &IDWriteFactory, size: f32 ) -> Result< IDWriteTextFormat > {
		unsafe {
			let format = factory.CreateTextFormat( w!( "Microsoft YaHei UI" ), None, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, size, w!( "zh-CN" ) )?;
			format.SetTextAlignment( DWRITE_TEXT_ALIGNMENT_LEADING )?;
			format.SetParagraphAlignment( DWRITE_PARAGRAPH_ALIGNMENT_NEAR )?;
			Ok( format )
		}
	}


	fn draw_text( target: &ID2D1DeviceContext, brush: &ID2D1SolidColorBrush, format: &IDWriteTextFormat, text: &str, rect: &D2D_RECT_F ) {
		let utf16: Vec< u16 > = text.encode_utf16().collect();
		unsafe { target.DrawText( &utf16, format, rect, brush, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL ); }
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
