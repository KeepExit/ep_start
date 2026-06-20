//! ::  Project Path  ->  ep_start :: backdrop_capture.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 00:48 周日


use platform::MonitorGeometry;
use windows::Graphics::Capture::{ Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession };
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{ HMODULE, HWND, RECT };
use windows::Win32::Graphics::Direct2D::Common::{ D2D1_ALPHA_MODE_IGNORE, D2D1_COMPOSITE_MODE_SOURCE_COPY, D2D1_PIXEL_FORMAT, D2D_RECT_F };
use windows::Win32::Graphics::Direct2D::{ CLSID_D2D1GaussianBlur, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION, D2D1_INTERPOLATION_MODE_LINEAR, D2D1_PROPERTY_TYPE_FLOAT, D2D1CreateDevice, ID2D1Bitmap1, ID2D1DeviceContext, ID2D1Effect };
use windows::Win32::Graphics::Direct3D::{ D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL };
use windows::Win32::Graphics::Direct3D11::{ D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device, ID3D11Texture2D };
use windows::Win32::Graphics::Dxgi::Common::{ DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC };
use windows::Win32::Graphics::Dxgi::{ DXGI_PRESENT, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIAdapter, IDXGIDevice, IDXGIFactory2, IDXGIOutput, IDXGISurface, IDXGISwapChain1 };
use windows::Win32::System::WinRT::Direct3D11::{ CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess };
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::core::{ Interface, Result as WindowsResult, factory };


pub struct DesktopCapture {
	frame_pool: Direct3D11CaptureFramePool,
	session: GraphicsCaptureSession,
	_device: ID3D11Device,
	swap_chain: IDXGISwapChain1,
	d2d_context: ID2D1DeviceContext,
	_target_bitmap: ID2D1Bitmap1,
	blur_effect: ID2D1Effect,
	source_rect: D2D_RECT_F,
	width: i32,
	height: i32,
}


impl DesktopCapture {
	pub fn create( hwnd: HWND, source_rect: RECT, geometry: &MonitorGeometry ) -> Result< Self, String > {
		unsafe { Self::create_native( hwnd, source_rect, geometry ) }.map_err( |error| format!( "创建桌面实时高斯背景失败：{}", error ) )
	}


	unsafe fn create_native( hwnd: HWND, source_window_rect: RECT, geometry: &MonitorGeometry ) -> WindowsResult< Self > {
		let interop: IGraphicsCaptureItemInterop = factory::< GraphicsCaptureItem, IGraphicsCaptureItemInterop >()?;
		let source = unsafe { windows::Win32::UI::WindowsAndMessaging::FindWindowW( windows::core::w!( "Progman" ), windows::core::PCWSTR::null() )? };
		let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow( source )? };
		let item_size = item.Size()?;
		let mut device = None;
		let mut feature_level = D3D_FEATURE_LEVEL::default();
		unsafe { D3D11CreateDevice( None::< &IDXGIAdapter >, D3D_DRIVER_TYPE_HARDWARE, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some( &mut device ), Some( &mut feature_level ), None )?; }
		let device = device.unwrap();
		let dxgi_device: IDXGIDevice = device.cast()?;
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
		let adapter = unsafe { dxgi_device.GetAdapter()? };
		let factory: IDXGIFactory2 = unsafe { adapter.GetParent()? };
		let description = DXGI_SWAP_CHAIN_DESC1 { Width: width as u32, Height: height as u32, Format: DXGI_FORMAT_B8G8R8A8_UNORM, Stereo: false.into(), SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 }, BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT, BufferCount: 2, Scaling: DXGI_SCALING_STRETCH, SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD, AlphaMode: DXGI_ALPHA_MODE_IGNORE, Flags: 0 };
		let swap_chain = unsafe { factory.CreateSwapChainForHwnd( &device, hwnd, &description, None, None::< &IDXGIOutput > )? };
		let target_surface: IDXGISurface = unsafe { swap_chain.GetBuffer( 0 )? };
		let d2d_device = unsafe { D2D1CreateDevice( &dxgi_device, None )? };
		let d2d_context = unsafe { d2d_device.CreateDeviceContext( D2D1_DEVICE_CONTEXT_OPTIONS_NONE )? };
		let bitmap_properties = D2D1_BITMAP_PROPERTIES1 { pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_IGNORE }, dpiX: 96.0, dpiY: 96.0, bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW, ..Default::default() };
		let target_bitmap = unsafe { d2d_context.CreateBitmapFromDxgiSurface( &target_surface, Some( &bitmap_properties ) )? };
		let blur_effect = unsafe { d2d_context.CreateEffect( &CLSID_D2D1GaussianBlur )? };
		unsafe { d2d_context.SetTarget( &target_bitmap ); }
		session.StartCapture()?;
		Ok( Self { frame_pool, session, _device: device, swap_chain, d2d_context, _target_bitmap: target_bitmap, blur_effect, source_rect: D2D_RECT_F { left: crop_left as f32, top: crop_top as f32, right: ( crop_left + width ) as f32, bottom: ( crop_top + height ) as f32 }, width, height } )
	}


	pub fn matches( &self, geometry: &MonitorGeometry ) -> bool {
		self.width == geometry.work_width() && self.height == geometry.work_height()
	}


	pub fn set_blur( &mut self, blur_percent: u8 ) {
		let deviation = blur_percent.min( 100 ) as f32 / 100.0 * 36.0;
		unsafe { let _ = self.blur_effect.SetValue( D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION.0 as u32, D2D1_PROPERTY_TYPE_FLOAT, &deviation.to_ne_bytes() ); }
	}


	pub fn present_next_frame( &mut self ) -> bool {
		let Some( frame ) = self.frame_pool.TryGetNextFrame().ok() else { return false; };
		let rendered = ( || -> WindowsResult< () > {
			let surface = frame.Surface()?;
			let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
			let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };
			let input_surface: IDXGISurface = texture.cast()?;
			let input_bitmap = unsafe { self.d2d_context.CreateBitmapFromDxgiSurface( &input_surface, None )? };
			unsafe {
				self.blur_effect.SetInput( 0, &input_bitmap, true );
				self.d2d_context.BeginDraw();
				self.d2d_context.Clear( None );
				let output = self.blur_effect.GetOutput()?;
				self.d2d_context.DrawImage( &output, None, Some( &self.source_rect ), D2D1_INTERPOLATION_MODE_LINEAR, D2D1_COMPOSITE_MODE_SOURCE_COPY );
				self.d2d_context.EndDraw( None, None )?;
				self.swap_chain.Present( 1, DXGI_PRESENT( 0 ) ).ok()?;
			}
			Ok( () )
		} )().is_ok();
		let _ = frame.Close();
		rendered
	}
}


impl Drop for DesktopCapture {
	fn drop( &mut self ) {
		let _ = self.session.Close();
		let _ = self.frame_pool.Close();
	}
}
