//! ::  Project Path  ->  ep_start :: backdrop_capture.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 00:48 周日


use platform::MonitorGeometry;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{ AtomicBool, Ordering };
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{ Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession };
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{ HMODULE, HWND, LPARAM, RECT, WPARAM };
use windows::Win32::Graphics::Direct2D::Common::{ D2D1_ALPHA_MODE_IGNORE, D2D1_BORDER_MODE_HARD, D2D1_COMPOSITE_MODE_SOURCE_COPY, D2D1_PIXEL_FORMAT, D2D_RECT_F };
use windows::Win32::Graphics::Direct2D::{ CLSID_D2D1GaussianBlur, CLSID_D2D1Scale, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_GAUSSIANBLUR_OPTIMIZATION_SPEED, D2D1_GAUSSIANBLUR_PROP_BORDER_MODE, D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION, D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION, D2D1_INTERPOLATION_MODE_LINEAR, D2D1_PROPERTY_TYPE_ENUM, D2D1_PROPERTY_TYPE_FLOAT, D2D1_PROPERTY_TYPE_VECTOR2, D2D1_SCALE_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC, D2D1_SCALE_PROP_INTERPOLATION_MODE, D2D1_SCALE_PROP_SCALE, D2D1CreateDevice, ID2D1Bitmap1, ID2D1DeviceContext, ID2D1Effect };
use windows::Win32::Graphics::Direct3D::{ D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL };
use windows::Win32::Graphics::Direct3D11::{ D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device, ID3D11Texture2D };
use windows::Win32::Graphics::Dxgi::Common::{ DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC };
use windows::Win32::Graphics::Dxgi::{ DXGI_PRESENT_DO_NOT_WAIT, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIAdapter, IDXGIDevice, IDXGIDevice1, IDXGIFactory2, IDXGIOutput, IDXGISurface, IDXGISwapChain1 };
use windows::Win32::System::WinRT::Direct3D11::{ CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess };
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::core::{ Interface, Result as WindowsResult, factory };


const BLUR_DOWNSAMPLE_DIVISOR: i32 = 4;


pub struct DesktopCapture {
	frame_pool: Direct3D11CaptureFramePool,
	session: GraphicsCaptureSession,
	_device: ID3D11Device,
	swap_chain: IDXGISwapChain1,
	d2d_context: ID2D1DeviceContext,
	_target_bitmap: ID2D1Bitmap1,
	scale_effect: ID2D1Effect,
	blur_effect: ID2D1Effect,
	source_rect: D2D_RECT_F,
	frame_pending: Arc< AtomicBool >,
	frame_arrived_token: i64,
	render_scale: f32,
	width: i32,
	height: i32,
}


impl DesktopCapture {
	pub fn create( hwnd: HWND, source_rect: RECT, geometry: &MonitorGeometry, notify_hwnd: HWND, notify_message: u32 ) -> Result< Self, String > {
		unsafe { Self::create_native( hwnd, source_rect, geometry, notify_hwnd, notify_message ) }.map_err( |error| format!( "创建桌面实时高斯背景失败：{}", error ) )
	}


	unsafe fn create_native( hwnd: HWND, source_window_rect: RECT, geometry: &MonitorGeometry, notify_hwnd: HWND, notify_message: u32 ) -> WindowsResult< Self > {
		let interop: IGraphicsCaptureItemInterop = factory::< GraphicsCaptureItem, IGraphicsCaptureItemInterop >()?;
		let source = unsafe { windows::Win32::UI::WindowsAndMessaging::FindWindowW( windows::core::w!( "Progman" ), windows::core::PCWSTR::null() )? };
		let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow( source )? };
		let item_size = item.Size()?;
		let mut device = None;
		let mut feature_level = D3D_FEATURE_LEVEL::default();
		unsafe { D3D11CreateDevice( None::< &IDXGIAdapter >, D3D_DRIVER_TYPE_HARDWARE, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some( &mut device ), Some( &mut feature_level ), None )?; }
		let device = device.unwrap();
		let dxgi_device: IDXGIDevice = device.cast()?;
		let dxgi_device1: IDXGIDevice1 = dxgi_device.cast()?;
		unsafe { dxgi_device1.SetMaximumFrameLatency( 1 )?; }
		let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice( &dxgi_device )? };
		let winrt_device: IDirect3DDevice = inspectable.cast()?;
		let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded( &winrt_device, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, item_size )?;
		let session = frame_pool.CreateCaptureSession( &item )?;
		let _ = session.SetIsCursorCaptureEnabled( false );
		let _ = session.SetIsBorderRequired( false );
		let crop_left = ( geometry.work_rect.left - source_window_rect.left ).max( 0 ).min( item_size.Width.saturating_sub( 1 ) );
		let crop_top = ( geometry.work_rect.top - source_window_rect.top ).max( 0 ).min( item_size.Height.saturating_sub( 1 ) );
		let width = geometry.work_width().max( 1 ).min( item_size.Width - crop_left );
		let height = geometry.work_height().max( 1 ).min( item_size.Height - crop_top );
		let render_width = ( width + BLUR_DOWNSAMPLE_DIVISOR - 1 ) / BLUR_DOWNSAMPLE_DIVISOR;
		let render_height = ( height + BLUR_DOWNSAMPLE_DIVISOR - 1 ) / BLUR_DOWNSAMPLE_DIVISOR;
		let scale_x = render_width as f32 / width as f32;
		let scale_y = render_height as f32 / height as f32;
		let render_scale = scale_x.min( scale_y );
		let adapter = unsafe { dxgi_device.GetAdapter()? };
		let factory: IDXGIFactory2 = unsafe { adapter.GetParent()? };
		let description = DXGI_SWAP_CHAIN_DESC1 { Width: render_width as u32, Height: render_height as u32, Format: DXGI_FORMAT_B8G8R8A8_UNORM, Stereo: false.into(), SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 }, BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT, BufferCount: 2, Scaling: DXGI_SCALING_STRETCH, SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD, AlphaMode: DXGI_ALPHA_MODE_IGNORE, Flags: 0 };
		let swap_chain = unsafe { factory.CreateSwapChainForHwnd( &device, hwnd, &description, None, None::< &IDXGIOutput > )? };
		let target_surface: IDXGISurface = unsafe { swap_chain.GetBuffer( 0 )? };
		let d2d_device = unsafe { D2D1CreateDevice( &dxgi_device, None )? };
		let d2d_context = unsafe { d2d_device.CreateDeviceContext( D2D1_DEVICE_CONTEXT_OPTIONS_NONE )? };
		let bitmap_properties = D2D1_BITMAP_PROPERTIES1 { pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_IGNORE }, dpiX: 96.0, dpiY: 96.0, bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW, ..Default::default() };
		let target_bitmap = unsafe { d2d_context.CreateBitmapFromDxgiSurface( &target_surface, Some( &bitmap_properties ) )? };
		let scale_effect = unsafe { d2d_context.CreateEffect( &CLSID_D2D1Scale )? };
		let blur_effect = unsafe { d2d_context.CreateEffect( &CLSID_D2D1GaussianBlur )? };
		let scale = vector2_bytes( scale_x, scale_y );
		unsafe {
			scale_effect.SetValue( D2D1_SCALE_PROP_SCALE.0 as u32, D2D1_PROPERTY_TYPE_VECTOR2, &scale )?;
			scale_effect.SetValue( D2D1_SCALE_PROP_INTERPOLATION_MODE.0 as u32, D2D1_PROPERTY_TYPE_ENUM, &D2D1_SCALE_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC.0.to_ne_bytes() )?;
			blur_effect.SetValue( D2D1_GAUSSIANBLUR_PROP_OPTIMIZATION.0 as u32, D2D1_PROPERTY_TYPE_ENUM, &D2D1_GAUSSIANBLUR_OPTIMIZATION_SPEED.0.to_ne_bytes() )?;
			blur_effect.SetValue( D2D1_GAUSSIANBLUR_PROP_BORDER_MODE.0 as u32, D2D1_PROPERTY_TYPE_ENUM, &D2D1_BORDER_MODE_HARD.0.to_ne_bytes() )?;
			let scaled_output = scale_effect.GetOutput()?;
			blur_effect.SetInput( 0, &scaled_output, true );
			d2d_context.SetTarget( &target_bitmap );
		}
		let frame_pending = Arc::new( AtomicBool::new( false ) );
		let callback_pending = frame_pending.clone();
		let notify_handle = notify_hwnd.0 as isize;
		let handler = TypedEventHandler::new( move |_, _| {
			let notify_hwnd = HWND( notify_handle as *mut c_void );
			if !callback_pending.swap( true, Ordering::AcqRel ) { unsafe { let _ = PostMessageW( Some( notify_hwnd ), notify_message, WPARAM( 0 ), LPARAM( 0 ) ); } }
			Ok( () )
		} );
		let frame_arrived_token = frame_pool.FrameArrived( &handler )?;
		session.StartCapture()?;
		Ok( Self { frame_pool, session, _device: device, swap_chain, d2d_context, _target_bitmap: target_bitmap, scale_effect, blur_effect, source_rect: D2D_RECT_F { left: crop_left as f32 * scale_x, top: crop_top as f32 * scale_y, right: ( crop_left + width ) as f32 * scale_x, bottom: ( crop_top + height ) as f32 * scale_y }, frame_pending, frame_arrived_token, render_scale, width, height } )
	}


	pub fn matches( &self, geometry: &MonitorGeometry ) -> bool {
		self.width == geometry.work_width() && self.height == geometry.work_height()
	}


	pub fn set_blur( &mut self, blur_percent: u8 ) {
		let deviation = blur_percent.min( 100 ) as f32 / 100.0 * 36.0 * self.render_scale;
		unsafe { let _ = self.blur_effect.SetValue( D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION.0 as u32, D2D1_PROPERTY_TYPE_FLOAT, &deviation.to_ne_bytes() ); }
	}


	pub fn present_next_frame( &mut self ) -> bool {
		self.frame_pending.store( false, Ordering::Release );
		let Some( mut frame ) = self.frame_pool.TryGetNextFrame().ok() else { return false; };
		while let Ok( next ) = self.frame_pool.TryGetNextFrame() { let _ = frame.Close(); frame = next; }
		let rendered = ( || -> WindowsResult< () > {
			let surface = frame.Surface()?;
			let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
			let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };
			let input_surface: IDXGISurface = texture.cast()?;
			let input_bitmap = unsafe { self.d2d_context.CreateBitmapFromDxgiSurface( &input_surface, None )? };
			unsafe {
				self.scale_effect.SetInput( 0, &input_bitmap, true );
				self.d2d_context.BeginDraw();
				self.d2d_context.Clear( None );
				let output = self.blur_effect.GetOutput()?;
				self.d2d_context.DrawImage( &output, None, Some( &self.source_rect ), D2D1_INTERPOLATION_MODE_LINEAR, D2D1_COMPOSITE_MODE_SOURCE_COPY );
				self.d2d_context.EndDraw( None, None )?;
				self.swap_chain.Present( 0, DXGI_PRESENT_DO_NOT_WAIT ).ok()?;
			}
			Ok( () )
		} )().is_ok();
		let _ = frame.Close();
		rendered
	}
}


impl Drop for DesktopCapture {
	fn drop( &mut self ) {
		let _ = self.frame_pool.RemoveFrameArrived( self.frame_arrived_token );
		let _ = self.session.Close();
		let _ = self.frame_pool.Close();
	}
}


fn vector2_bytes( x: f32, y: f32 ) -> [ u8; 8 ] {
	let mut bytes = [ 0u8; 8 ];
	bytes[ 0..4 ].copy_from_slice( &x.to_ne_bytes() );
	bytes[ 4..8 ].copy_from_slice( &y.to_ne_bytes() );
	bytes
}
