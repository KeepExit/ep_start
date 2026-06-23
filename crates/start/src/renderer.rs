//! ::  Project Path  ->  ep_start :: renderer.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 02:20 周六


use crate::animation::AnimationFrame;
use crate::config::{ StartConfig, Tile };
use crate::context_menu::{ ContextMenuNode, ContextMenuPanelVisual, ContextMenuSelection, ContextMenuVisual };
use crate::layout::{ DragSource, DragVisual, FolderTileAddress, TileAddress, TileDropVisual, TileLayout, indicator_ease, interpolate_rect, reflow_ease, smooth_step };
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem::size_of;
use windows_numerics::{ Matrix3x2, Vector2 };
use windows::Win32::Foundation::{ HMODULE, HWND };
use windows::Win32::Graphics::Direct2D::Common::{ D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_GRADIENT_STOP, D2D1_PIXEL_FORMAT };
use windows::Win32::Graphics::Direct2D::{ D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1, D2D1_BUFFER_PRECISION_8BPC_UNORM, D2D1_COLOR_INTERPOLATION_MODE_PREMULTIPLIED, D2D1_COLOR_SPACE_SRGB, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_EXTEND_MODE_CLAMP, D2D1_INTERPOLATION_MODE_LINEAR, D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES, D2D1_ROUNDED_RECT, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateDevice, ID2D1Bitmap1, ID2D1Device, ID2D1DeviceContext, ID2D1RadialGradientBrush, ID2D1SolidColorBrush };
use windows::Win32::Graphics::Direct3D::{ D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL };
use windows::Win32::Graphics::Direct3D11::{ D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device };
use windows::Win32::Graphics::DirectComposition::{ DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual };
use windows::Win32::Graphics::DirectWrite::{ DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_METRICS, DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat };
use windows::Win32::Graphics::Dxgi::Common::{ DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC };
use windows::Win32::Graphics::Dxgi::{ DXGI_PRESENT, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIAdapter, IDXGIDevice, IDXGIDevice1, IDXGIFactory2, IDXGIOutput, IDXGISurface, IDXGISwapChain1 };
use windows::Win32::Graphics::Imaging::{ CLSID_WICImagingFactory, IWICImagingFactory };
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::System::Com::{ CLSCTX_INPROC_SERVER, CoCreateInstance };
use windows::Win32::UI::Shell::{ SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW };
use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;
use windows::core::{ Interface, PCWSTR, Result, w };


const TITLE_INSET: f32 = 10.0;
const PLACEHOLDER_SIZE: f32 = 52.0;
const TILE_CORNER_RADIUS: f32 = 5.0;
const REVEAL_RADIUS: f32 = 150.0;
const TILE_BACKGROUND_COLOR: D2D1_COLOR_F = D2D1_COLOR_F { r: 0.19, g: 0.19, b: 0.20, a: 1.0 };


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
	reveal_brush: ID2D1RadialGradientBrush,
	icon_cache: RefCell< HashMap< String, ID2D1Bitmap1 > >,
	size: D2D_SIZE_U,
}


pub struct Renderer {
	write_factory: IDWriteFactory,
	group_text_format: IDWriteTextFormat,
	tile_text_format: IDWriteTextFormat,
	icon_text_format: IDWriteTextFormat,
	menu_text_format: IDWriteTextFormat,
	menu_icon_text_format: IDWriteTextFormat,
	device: Option< DeviceResources >,
}


impl Renderer {
	pub fn new() -> Result< Self > {
		unsafe {
			let write_factory = DWriteCreateFactory::< IDWriteFactory >( DWRITE_FACTORY_TYPE_SHARED )?;
			let group_text_format = Self::create_text_format( &write_factory, 16.0 )?;
			let tile_text_format = Self::create_text_format( &write_factory, 14.0 )?;
			let icon_text_format = Self::create_text_format( &write_factory, 32.0 )?;
			let menu_text_format = Self::create_text_format( &write_factory, 15.0 )?;
			let menu_icon_text_format = Self::create_font_text_format( &write_factory, w!( "Segoe Fluent Icons" ), 16.0 )?;
			Ok( Self { write_factory, group_text_format, tile_text_format, icon_text_format, menu_text_format, menu_icon_text_format, device: None } )
		}
	}


	pub fn paint( &mut self, hwnd: HWND, size: D2D_SIZE_U, dpi: f32, config: &StartConfig, layout: &TileLayout, hovered: Option< TileAddress >, hovered_folder: Option< FolderTileAddress >, renaming_bar: Option< ( usize, bool ) >, pointer: Option< ( f32, f32 ) >, rounded_tiles: bool, rounded_tile_bars: bool, tile_background_opacity: f32, tile_bar_background_opacity: f32, tile_creation_progress: Option< f32 >, drag: Option< DragVisual >, drop: Option< TileDropVisual >, drag_source_config: Option< &StartConfig >, drag_source_layout: Option< &TileLayout >, context_menu: Option< ContextMenuVisual<'_> >, frame: &AnimationFrame ) -> Result< () > {
		self.ensure_device_resources( hwnd, size, dpi )?;
		let device = self.device.as_ref().expect( "渲染资源应已创建" );
		unsafe {
			device.target.BeginDraw();
			device.target.Clear( Some( &D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 } ) );
			if let Some( pointer ) = pointer { device.reveal_brush.SetCenter( Vector2 { X: pointer.0, Y: pointer.1 } ); }
			for bar_region in &layout.bars {
				if matches!( drag.as_ref().map( |value| value.preview_source ), Some( DragSource::Bar( bar_index ) ) if bar_index == bar_region.bar_index ) { continue; }
				let progress = frame.group_progress( bar_region.bar_index );
				if progress <= 0.0 { continue; }
				let bar = &config.bars[ bar_region.bar_index ];
				device.tile_brush.SetColor( &D2D1_COLOR_F { r: 0.16, g: 0.16, b: 0.17, a: 1.0 } );
				device.tile_brush.SetOpacity( progress * tile_bar_background_opacity );
				device.target.SetTransform( &animation_transform( &bar_region.rect, progress, 18.0 ) );
				fill_bar( &device.target, &device.tile_brush, &bar_region.rect, rounded_tile_bars );
				let editing = renaming_bar.filter( |( bar_index, _ )| *bar_index == bar_region.bar_index );
				let title_input = D2D_RECT_F { left: bar_region.title_rect.left, top: bar_region.title_rect.top + 1.0, right: bar_region.title_rect.right - 38.0, bottom: bar_region.title_rect.bottom - 1.0 };
				if editing.is_some() {
					device.tile_brush.SetColor( &D2D1_COLOR_F { r: 0.09, g: 0.09, b: 0.10, a: 1.0 } );
					device.tile_brush.SetOpacity( 0.92 * progress );
					device.target.FillRoundedRectangle( &D2D1_ROUNDED_RECT { rect: title_input, radiusX: 4.0, radiusY: 4.0 }, &device.tile_brush );
					device.hover_brush.SetOpacity( 0.72 * progress );
					device.target.DrawRoundedRectangle( &D2D1_ROUNDED_RECT { rect: title_input, radiusX: 4.0, radiusY: 4.0 }, &device.hover_brush, 1.0, None );
				}
				let title_text = D2D_RECT_F { left: bar_region.title_rect.left + TITLE_INSET, top: bar_region.title_rect.top + 6.0, right: title_input.right - 8.0, bottom: bar_region.title_rect.bottom };
				device.text_brush.SetOpacity( progress );
				Self::draw_text( &device.target, &device.text_brush, &self.group_text_format, &bar.title, &title_text );
				if editing.is_some_and( |( _, caret_visible )| caret_visible ) {
					let caret_x = ( title_text.left + self.text_width( &bar.title, &self.group_text_format, title_text.right - title_text.left, title_text.bottom - title_text.top ) + 1.0 ).min( title_text.right - 1.0 );
					device.hover_brush.SetOpacity( progress );
					device.target.FillRectangle( &D2D_RECT_F { left: caret_x, top: title_text.top + 3.0, right: caret_x + 1.5, bottom: title_text.bottom - 5.0 }, &device.hover_brush );
				}
				self.draw_bar_handle( device, &bar_region.title_rect, bar.locked, progress );
			}
			for ( render_index, tile_region ) in layout.tiles.iter().enumerate() {
				if matches!( drag.as_ref().map( |value| value.preview_source ), Some( DragSource::Tile( address ) ) if address == tile_region.address ) || matches!( drag.as_ref().map( |value| value.preview_source ), Some( DragSource::Bar( bar_index ) ) if bar_index == tile_region.address.bar_index ) { continue; }
				let progress = frame.tile_progress( render_index );
				let opacity = frame.tile_opacity( render_index );
				if opacity <= 0.0 { continue; }
				let tile = &config.bars[ tile_region.address.bar_index ].tiles[ tile_region.address.tile_index ];
				if drop.is_some_and( |value| value.runtime_id == tile.runtime_id() ) { continue; }
				let rect = drag.as_ref().and_then( |drag| drag.reflow_origins.get( &tile.runtime_id() ).copied().map( |source| interpolate_rect( source, tile_region.rect, reflow_ease( drag.reflow_progress ) ) ) ).unwrap_or( tile_region.rect );
				device.target.SetTransform( &animation_transform( &rect, progress, 28.0 ) );
				self.draw_tile_surface( device, tile, &rect, opacity, tile_background_opacity, rounded_tiles );
				if hovered == Some( tile_region.address ) { device.hover_brush.SetOpacity( 0.10 * opacity ); fill_tile( &device.target, &device.hover_brush, &rect, rounded_tiles ); }
				if let Some( pointer ) = pointer { draw_reveal_border( device, &rect, pointer, opacity, rounded_tiles ); }
			}
			self.draw_folder_panel( device, config, layout, hovered_folder, rounded_tiles, tile_background_opacity );
			if let Some( drag ) = drag { self.draw_drag_visual( device, drag_source_config.unwrap_or( config ), drag_source_layout.unwrap_or( layout ), config, layout, &drag, rounded_tiles, rounded_tile_bars, tile_background_opacity, tile_bar_background_opacity ); }
			if let Some( drop ) = drop { self.draw_drop_visual( device, config, &drop, rounded_tiles, tile_background_opacity ); }
			if let Some( progress ) = tile_creation_progress { self.draw_tile_creation( device, size.width as f32 * 96.0 / dpi, size.height as f32 * 96.0 / dpi, smooth_step( progress ) ); }
			if let Some( menu ) = context_menu { self.draw_context_menu( device, &menu ); }
			device.target.SetTransform( &identity_transform() );
			if let Err( error ) = device.target.EndDraw( None, None ) {
				self.device = None;
				return Err( error );
			}
			device.swap_chain.Present( 1, DXGI_PRESENT( 0 ) ).ok()?;
		}
		Ok( () )
	}


	fn draw_tile_surface( &self, device: &DeviceResources, tile: &Tile, rect: &D2D_RECT_F, opacity: f32, tile_background_opacity: f32, rounded_tiles: bool ) {
		unsafe {
			device.tile_brush.SetColor( &parse_color( &tile.color ) );
			device.tile_brush.SetOpacity( opacity * tile_background_opacity );
			device.text_brush.SetOpacity( opacity );
			fill_tile( &device.target, &device.tile_brush, rect, rounded_tiles );
		}
		self.draw_tile_icon( device, tile, rect );
		if rect.bottom - rect.top >= 72.0 && rect.right - rect.left >= 78.0 { let title_rect = D2D_RECT_F { left: rect.left + TITLE_INSET, top: rect.bottom - 31.0, right: rect.right - TITLE_INSET, bottom: rect.bottom - 8.0 }; Self::draw_text( &device.target, &device.text_brush, &self.tile_text_format, &tile.title, &title_rect ); }
	}


	fn draw_bar_handle( &self, device: &DeviceResources, title_rect: &D2D_RECT_F, locked: bool, opacity: f32 ) {
		let center_x = title_rect.right - 17.0;
		let center_y = ( title_rect.top + title_rect.bottom ) * 0.5;
		if locked {
			unsafe { device.text_brush.SetOpacity( 0.48 * opacity ); }
			Self::draw_text( &device.target, &device.text_brush, &self.menu_icon_text_format, "\u{E72E}", &D2D_RECT_F { left: center_x - 9.0, top: center_y - 9.0, right: center_x + 9.0, bottom: center_y + 11.0 } );
			return;
		}
		unsafe {
			device.hover_brush.SetOpacity( 0.76 * opacity );
			for offset in [ -5.0, 0.0, 5.0 ] { device.target.DrawLine( Vector2 { X: center_x - 8.0, Y: center_y + offset }, Vector2 { X: center_x + 8.0, Y: center_y + offset }, &device.hover_brush, 1.5, None ); }
		}
	}


	fn draw_drop_visual( &self, device: &DeviceResources, config: &StartConfig, drop: &TileDropVisual, rounded_tiles: bool, tile_background_opacity: f32 ) {
		let Some( tile ) = config.bars.iter().flat_map( |bar| &bar.tiles ).find( |tile| tile.runtime_id() == drop.runtime_id ) else { return; };
		let progress = indicator_ease( drop.progress );
		let rect = interpolate_rect( drop.from_rect, drop.to_rect, progress );
		unsafe {
			device.target.SetTransform( &identity_transform() );
			self.draw_tile_surface( device, tile, &rect, 1.0, tile_background_opacity, rounded_tiles );
			device.hover_brush.SetOpacity( 0.40 + smooth_step( drop.progress ) * 0.34 );
			if rounded_tiles { device.target.DrawRoundedRectangle( &D2D1_ROUNDED_RECT { rect, radiusX: TILE_CORNER_RADIUS, radiusY: TILE_CORNER_RADIUS }, &device.hover_brush, 2.0, None ); } else { device.target.DrawRectangle( &rect, &device.hover_brush, 2.0, None ); }
		}
	}


	fn draw_tile_icon( &self, device: &DeviceResources, tile: &Tile, rect: &D2D_RECT_F ) {
		let center_x = ( rect.left + rect.right ) * 0.5;
		let width = rect.right - rect.left;
		let height = rect.bottom - rect.top;
		let icon_size = PLACEHOLDER_SIZE.min( ( width - 12.0 ).max( 12.0 ) ).min( ( height - 18.0 ).max( 12.0 ) );
		let icon_top = rect.top + ( height - icon_size ) * if height >= 72.0 { 0.38 } else { 0.5 };
		let icon_rect = D2D_RECT_F { left: center_x - icon_size * 0.5, top: icon_top, right: center_x + icon_size * 0.5, bottom: icon_top + icon_size };
		if !tile.icon_source.is_empty() {
			if let Some( bitmap ) = load_program_icon( device, &tile.icon_source ) { unsafe { let _ = device.target.DrawBitmap( &bitmap, Some( &icon_rect ), 1.0, D2D1_INTERPOLATION_MODE_LINEAR, None, None ); } return; }
		}
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


	fn draw_drag_visual( &self, device: &DeviceResources, source_config: &StartConfig, source_layout: &TileLayout, preview_config: &StartConfig, preview_layout: &TileLayout, drag: &DragVisual, rounded_tiles: bool, rounded_tile_bars: bool, tile_background_opacity: f32, tile_bar_background_opacity: f32 ) {
		unsafe {
			device.target.SetTransform( &identity_transform() );
			let target_rect = match drag.preview_source { DragSource::Tile( address ) => preview_layout.tile_rect( address ), DragSource::Bar( _ ) => preview_layout.drop_rect( drag.target ) };
			if let Some( target_rect ) = target_rect {
				let entrance = smooth_step( drag.reflow_progress );
				let settle = indicator_ease( drag.reflow_progress );
				let indicator_rect = if let DragSource::Tile( address ) = drag.preview_source {
					let tile = &preview_config.bars[ address.bar_index ].tiles[ address.tile_index ];
					let source = drag.reflow_origins.get( &tile.runtime_id() ).copied().unwrap_or( target_rect );
					interpolate_rect( source, target_rect, settle )
				} else { target_rect };
				device.tile_brush.SetColor( &TILE_BACKGROUND_COLOR );
				device.tile_brush.SetOpacity( ( 0.18 + entrance * 0.10 ) * tile_background_opacity );
				if matches!( drag.preview_source, DragSource::Tile( _ ) ) { fill_tile( &device.target, &device.tile_brush, &indicator_rect, rounded_tiles ); }
				device.hover_brush.SetOpacity( 0.50 + entrance * 0.34 );
				if matches!( drag.preview_source, DragSource::Tile( _ ) ) && rounded_tiles { device.target.DrawRoundedRectangle( &D2D1_ROUNDED_RECT { rect: indicator_rect, radiusX: TILE_CORNER_RADIUS, radiusY: TILE_CORNER_RADIUS }, &device.hover_brush, 2.0, None ); } else { device.target.DrawRectangle( &indicator_rect, &device.hover_brush, 2.0, None ); }
			}
			device.target.SetTransform( &translation_transform( drag.delta_x, drag.delta_y ) );
			match drag.source {
				DragSource::Tile( address ) => {
					{
						let rect = drag.origin_rect;
						let tile = &source_config.bars[ address.bar_index ].tiles[ address.tile_index ];
						device.tile_brush.SetColor( &parse_color( &tile.color ) );
						device.tile_brush.SetOpacity( tile_background_opacity );
						device.text_brush.SetOpacity( 0.9 );
						fill_tile( &device.target, &device.tile_brush, &rect, rounded_tiles );
						let title_rect = D2D_RECT_F { left: rect.left + TITLE_INSET, top: rect.bottom - 31.0, right: rect.right - TITLE_INSET, bottom: rect.bottom - 8.0 };
						Self::draw_text( &device.target, &device.text_brush, &self.tile_text_format, &tile.title, &title_rect );
					}
				}
				DragSource::Bar( bar_index ) => {
					if let Some( bar_region ) = source_layout.bars.iter().find( |bar| bar.bar_index == bar_index ) {
						let bar = &source_config.bars[ bar_index ];
						device.tile_brush.SetColor( &D2D1_COLOR_F { r: 0.16, g: 0.16, b: 0.17, a: 1.0 } );
						device.tile_brush.SetOpacity( tile_bar_background_opacity * 0.92 );
						fill_bar( &device.target, &device.tile_brush, &bar_region.rect, rounded_tile_bars );
						device.text_brush.SetOpacity( 0.9 );
						Self::draw_text( &device.target, &device.text_brush, &self.group_text_format, &bar.title, &D2D_RECT_F { left: bar_region.title_rect.left + TITLE_INSET, top: bar_region.title_rect.top + 6.0, right: bar_region.title_rect.right - 46.0, bottom: bar_region.title_rect.bottom } );
						self.draw_bar_handle( device, &bar_region.title_rect, bar.locked, 0.9 );
						for tile_region in source_layout.tiles.iter().filter( |tile| tile.address.bar_index == bar_index ) {
							let tile = &bar.tiles[ tile_region.address.tile_index ];
							device.tile_brush.SetColor( &parse_color( &tile.color ) );
							device.tile_brush.SetOpacity( tile_background_opacity * 0.88 );
							fill_tile( &device.target, &device.tile_brush, &tile_region.rect, rounded_tiles );
						}
					}
				}
			}
			device.target.SetTransform( &identity_transform() );
		}
	}


	fn draw_folder_panel( &self, device: &DeviceResources, config: &StartConfig, layout: &TileLayout, hovered: Option< FolderTileAddress >, rounded_tiles: bool, tile_background_opacity: f32 ) {
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
				device.tile_brush.SetOpacity( tile_background_opacity );
				device.text_brush.SetOpacity( 1.0 );
				fill_tile( &device.target, &device.tile_brush, &tile_region.rect, rounded_tiles );
				if hovered == Some( tile_region.address ) { device.hover_brush.SetOpacity( 0.14 ); fill_tile( &device.target, &device.hover_brush, &tile_region.rect, rounded_tiles ); }
				let title_rect = D2D_RECT_F { left: tile_region.rect.left + TITLE_INSET, top: tile_region.rect.bottom - 31.0, right: tile_region.rect.right - TITLE_INSET, bottom: tile_region.rect.bottom - 8.0 };
				Self::draw_text( &device.target, &device.text_brush, &self.tile_text_format, &tile.title, &title_rect );
			}
		}
	}


	fn draw_tile_creation( &self, device: &DeviceResources, width: f32, height: f32, progress: f32 ) {
		let panel_width = 460.0;
		let panel_height = 286.0;
		let left = ( width - panel_width ) * 0.5;
		let top = ( height - panel_height ) * 0.5;
		let panel = D2D_RECT_F { left, top, right: left + panel_width, bottom: top + panel_height };
		let rows = [ D2D_RECT_F { left: left + 28.0, top: top + 78.0, right: left + panel_width - 28.0, bottom: top + 132.0 }, D2D_RECT_F { left: left + 28.0, top: top + 142.0, right: left + panel_width - 28.0, bottom: top + 196.0 }, D2D_RECT_F { left: left + 28.0, top: top + 206.0, right: left + panel_width - 28.0, bottom: top + 260.0 } ];
		unsafe {
			device.target.SetTransform( &identity_transform() );
			device.tile_brush.SetColor( &D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.56 * progress } );
			device.tile_brush.SetOpacity( progress );
			device.target.FillRectangle( &D2D_RECT_F { left: 0.0, top: 0.0, right: width, bottom: height }, &device.tile_brush );
			device.target.SetTransform( &scale_transform( &panel, 0.94 + 0.06 * progress ) );
			device.tile_brush.SetColor( &D2D1_COLOR_F { r: 0.12, g: 0.12, b: 0.13, a: 0.98 } );
			device.tile_brush.SetOpacity( progress );
			device.target.FillRoundedRectangle( &D2D1_ROUNDED_RECT { rect: panel, radiusX: 12.0, radiusY: 12.0 }, &device.tile_brush );
			device.text_brush.SetOpacity( progress );
			Self::draw_text( &device.target, &device.text_brush, &self.group_text_format, "选择磁贴类型", &D2D_RECT_F { left: left + 28.0, top: top + 24.0, right: panel.right - 28.0, bottom: top + 58.0 } );
			for ( index, row ) in rows.iter().enumerate() {
				device.tile_brush.SetColor( &D2D1_COLOR_F { r: 0.22, g: 0.22, b: 0.23, a: if index == 0 { 0.94 } else { 0.54 } } );
				device.target.FillRoundedRectangle( &D2D1_ROUNDED_RECT { rect: *row, radiusX: 6.0, radiusY: 6.0 }, &device.tile_brush );
				device.text_brush.SetOpacity( if index == 0 { progress } else { 0.46 * progress } );
				Self::draw_text( &device.target, &device.text_brush, &self.tile_text_format, [ "程序", "网页（暂未开放）", "图片（暂未开放）" ][ index ], &D2D_RECT_F { left: row.left + 18.0, top: row.top + 15.0, right: row.right - 18.0, bottom: row.bottom } );
			}
			device.target.SetTransform( &identity_transform() );
		}
	}


	fn draw_context_menu( &self, device: &DeviceResources, menu: &ContextMenuVisual<'_> ) {
		self.draw_context_menu_panel_visual( device, &menu.root, menu.hovered, menu.pressed );
		if let Some( submenu ) = &menu.submenu { self.draw_context_menu_panel_visual( device, submenu, menu.hovered, menu.pressed ); }
		unsafe { device.target.SetTransform( &identity_transform() ); }
	}


	fn draw_context_menu_panel_visual( &self, device: &DeviceResources, panel: &ContextMenuPanelVisual<'_>, hovered: Option< ContextMenuSelection >, pressed: Option< ContextMenuSelection > ) {
		let progress = smooth_step( panel.progress );
		self.draw_context_menu_panel( device, &panel.layout.panel, progress );
		unsafe {
			device.tile_brush.SetColor( &D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 } );
			device.tile_brush.SetOpacity( 0.10 * progress );
			for separator in &panel.layout.separators { device.target.FillRectangle( separator, &device.tile_brush ); }
		}
		for row in &panel.layout.rows {
			let Some( ContextMenuNode::Item( item ) ) = panel.items.get( row.item_index ) else { continue; };
			let selection = ContextMenuSelection { panel: panel.panel, item_index: row.item_index };
			self.draw_context_menu_row( device, &row.rect, &item.icon, &item.label, !item.children.is_empty(), hovered == Some( selection ), pressed == Some( selection ), progress );
		}
	}


	fn draw_context_menu_panel( &self, device: &DeviceResources, panel: &D2D_RECT_F, progress: f32 ) {
		unsafe {
			device.target.SetTransform( &scale_transform( panel, 0.96 + 0.04 * progress ) );
			let shadow = D2D_RECT_F { left: panel.left - 3.0, top: panel.top, right: panel.right + 3.0, bottom: panel.bottom + 6.0 };
			device.tile_brush.SetColor( &D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 } );
			device.tile_brush.SetOpacity( 0.30 * progress );
			device.target.FillRoundedRectangle( &D2D1_ROUNDED_RECT { rect: shadow, radiusX: 10.0, radiusY: 10.0 }, &device.tile_brush );
			device.tile_brush.SetColor( &D2D1_COLOR_F { r: 0.125, g: 0.125, b: 0.135, a: 1.0 } );
			device.tile_brush.SetOpacity( 0.98 * progress );
			device.target.FillRoundedRectangle( &D2D1_ROUNDED_RECT { rect: *panel, radiusX: 8.0, radiusY: 8.0 }, &device.tile_brush );
			device.hover_brush.SetOpacity( 0.13 * progress );
			device.target.DrawRoundedRectangle( &D2D1_ROUNDED_RECT { rect: *panel, radiusX: 8.0, radiusY: 8.0 }, &device.hover_brush, 1.0, None );
		}
	}


	fn draw_context_menu_row( &self, device: &DeviceResources, row: &D2D_RECT_F, icon: &str, label: &str, chevron: bool, hovered: bool, pressed: bool, progress: f32 ) {
		unsafe {
			if hovered || pressed {
				device.tile_brush.SetColor( &D2D1_COLOR_F { r: 0.25, g: 0.25, b: 0.27, a: 1.0 } );
				device.tile_brush.SetOpacity( if pressed { 0.72 * progress } else { 0.54 * progress } );
				device.target.FillRoundedRectangle( &D2D1_ROUNDED_RECT { rect: *row, radiusX: 4.0, radiusY: 4.0 }, &device.tile_brush );
			}
			device.text_brush.SetOpacity( progress );
		}
		let has_icon = !icon.is_empty();
		if has_icon { Self::draw_text( &device.target, &device.text_brush, &self.menu_icon_text_format, icon, &D2D_RECT_F { left: row.left + 13.0, top: row.top + 11.0, right: row.left + 32.0, bottom: row.bottom } ); }
		let text_left = if has_icon { row.left + 43.0 } else { row.left + 13.0 };
		Self::draw_text( &device.target, &device.text_brush, &self.menu_text_format, label, &D2D_RECT_F { left: text_left, top: row.top + 9.0, right: row.right - 30.0, bottom: row.bottom } );
		if chevron { Self::draw_text( &device.target, &device.text_brush, &self.menu_icon_text_format, "\u{E76C}", &D2D_RECT_F { left: row.right - 23.0, top: row.top + 11.0, right: row.right - 7.0, bottom: row.bottom } ); }
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
			let reveal_stops = [ D2D1_GRADIENT_STOP { position: 0.0, color: D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.92 } }, D2D1_GRADIENT_STOP { position: 0.52, color: D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.48 } }, D2D1_GRADIENT_STOP { position: 1.0, color: D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 0.0 } } ];
			let reveal_stop_collection = target.CreateGradientStopCollection( &reveal_stops, D2D1_COLOR_SPACE_SRGB, D2D1_COLOR_SPACE_SRGB, D2D1_BUFFER_PRECISION_8BPC_UNORM, D2D1_EXTEND_MODE_CLAMP, D2D1_COLOR_INTERPOLATION_MODE_PREMULTIPLIED )?;
			let reveal_properties = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES { center: Vector2::default(), gradientOriginOffset: Vector2::default(), radiusX: REVEAL_RADIUS, radiusY: REVEAL_RADIUS };
			let reveal_brush = target.CreateRadialGradientBrush( &reveal_properties, None, &reveal_stop_collection )?;
			let composition_device = DCompositionCreateDevice::< _, IDCompositionDevice >( &dxgi_device )?;
			let composition_target = composition_device.CreateTargetForHwnd( hwnd, true )?;
			let composition_visual = composition_device.CreateVisual()?;
			composition_visual.SetContent( &swap_chain )?;
			composition_target.SetRoot( &composition_visual )?;
			composition_device.Commit()?;
			self.device = Some( DeviceResources { target, swap_chain, _target_bitmap: target_bitmap, _d3d_device: d3d_device, _d2d_device: d2d_device, _composition_device: composition_device, _composition_target: composition_target, _composition_visual: composition_visual, tile_brush, text_brush, hover_brush, reveal_brush, icon_cache: RefCell::new( HashMap::new() ), size } );
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
		Self::create_font_text_format( factory, w!( "Microsoft YaHei UI" ), size )
	}


	fn create_font_text_format( factory: &IDWriteFactory, family: PCWSTR, size: f32 ) -> Result< IDWriteTextFormat > {
		unsafe {
			let format = factory.CreateTextFormat( family, None, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, size, w!( "zh-CN" ) )?;
			format.SetTextAlignment( DWRITE_TEXT_ALIGNMENT_LEADING )?;
			format.SetParagraphAlignment( DWRITE_PARAGRAPH_ALIGNMENT_NEAR )?;
			Ok( format )
		}
	}


	fn draw_text( target: &ID2D1DeviceContext, brush: &ID2D1SolidColorBrush, format: &IDWriteTextFormat, text: &str, rect: &D2D_RECT_F ) {
		let utf16: Vec< u16 > = text.encode_utf16().collect();
		unsafe { target.DrawText( &utf16, format, rect, brush, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL ); }
	}


	fn text_width( &self, text: &str, format: &IDWriteTextFormat, maximum_width: f32, maximum_height: f32 ) -> f32 {
		let utf16: Vec< u16 > = text.encode_utf16().collect();
		let Ok( layout ) = ( unsafe { self.write_factory.CreateTextLayout( &utf16, format, maximum_width.max( 1.0 ), maximum_height.max( 1.0 ) ) } ) else { return 0.0; };
		let mut metrics = DWRITE_TEXT_METRICS::default();
		if unsafe { layout.GetMetrics( &mut metrics ) }.is_err() { return 0.0; }
		metrics.widthIncludingTrailingWhitespace
	}
}


fn fill_tile( target: &ID2D1DeviceContext, brush: &ID2D1SolidColorBrush, rect: &D2D_RECT_F, rounded: bool ) {
	unsafe {
		if rounded { target.FillRoundedRectangle( &D2D1_ROUNDED_RECT { rect: *rect, radiusX: TILE_CORNER_RADIUS, radiusY: TILE_CORNER_RADIUS }, brush ); } else { target.FillRectangle( rect, brush ); }
	}
}


fn fill_bar( target: &ID2D1DeviceContext, brush: &ID2D1SolidColorBrush, rect: &D2D_RECT_F, rounded: bool ) {
	unsafe {
		if rounded { target.FillRoundedRectangle( &D2D1_ROUNDED_RECT { rect: *rect, radiusX: 10.0, radiusY: 10.0 }, brush ); } else { target.FillRectangle( rect, brush ); }
	}
}


fn load_program_icon( device: &DeviceResources, source: &str ) -> Option< ID2D1Bitmap1 > {
	if let Some( cached ) = device.icon_cache.borrow().get( source ).cloned() { return Some( cached ); }
	let path: Vec< u16 > = source.encode_utf16().chain( [ 0 ] ).collect();
	let mut info = SHFILEINFOW::default();
	let result = unsafe { SHGetFileInfoW( PCWSTR( path.as_ptr() ), FILE_FLAGS_AND_ATTRIBUTES::default(), Some( &mut info ), size_of::< SHFILEINFOW >() as u32, SHGFI_ICON | SHGFI_LARGEICON ) };
	if result == 0 || info.hIcon.is_invalid() { return None; }
	let bitmap = ( || {
		let factory: IWICImagingFactory = unsafe { CoCreateInstance( &CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER ) }.ok()?;
		let wic = unsafe { factory.CreateBitmapFromHICON( info.hIcon ) }.ok()?;
		unsafe { device.target.CreateBitmapFromWicBitmap( &wic, None ) }.ok()
	} )();
	unsafe { let _ = DestroyIcon( info.hIcon ); }
	if let Some( bitmap ) = &bitmap { device.icon_cache.borrow_mut().insert( source.to_string(), bitmap.clone() ); }
	bitmap
}


fn draw_reveal_border( device: &DeviceResources, rect: &D2D_RECT_F, pointer: ( f32, f32 ), opacity: f32, rounded: bool ) {
	let closest_x = pointer.0.clamp( rect.left, rect.right );
	let closest_y = pointer.1.clamp( rect.top, rect.bottom );
	let distance = ( ( pointer.0 - closest_x ).powi( 2 ) + ( pointer.1 - closest_y ).powi( 2 ) ).sqrt();
	if distance >= REVEAL_RADIUS { return; }
	unsafe {
		device.reveal_brush.SetOpacity( opacity );
		if rounded { device.target.DrawRoundedRectangle( &D2D1_ROUNDED_RECT { rect: *rect, radiusX: TILE_CORNER_RADIUS, radiusY: TILE_CORNER_RADIUS }, &device.reveal_brush, 1.4, None ); } else { device.target.DrawRectangle( rect, &device.reveal_brush, 1.4, None ); }
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


fn scale_transform( rect: &D2D_RECT_F, scale: f32 ) -> Matrix3x2 {
	let center_x = ( rect.left + rect.right ) * 0.5;
	let center_y = ( rect.top + rect.bottom ) * 0.5;
	Matrix3x2 { M11: scale, M12: 0.0, M21: 0.0, M22: scale, M31: center_x * ( 1.0 - scale ), M32: center_y * ( 1.0 - scale ) }
}
