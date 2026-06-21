//! ::  Project Path  ->  ep_start :: window.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 02:20 周六


use crate::animation::{ AnimationController, VisibilityState };
use crate::backdrop::DesktopBackdrop;
use crate::config::{ ConfigStore, StartConfig };
use crate::launcher::ProgramLauncher;
use crate::layout::{ DragSource, DragVisual, DropTarget, FolderTileAddress, TileAddress, TileLayout };
use crate::overlay::OverlaySurface;
use crate::renderer::Renderer;
use crate::transition::DesktopTransition;
use configuration::{ StartPreferences, StartShortcut };
use platform::{ ForegroundActivation, ForegroundChangeObserver, GlobalInputAction, GlobalInputBinding, GlobalInputManager, GlobalStartShortcut, MonitorGeometry, show_error_dialog, trim_working_set };
use std::ffi::c_void;
use std::mem::size_of;
use windows::Win32::Foundation::{ HINSTANCE, HWND, LPARAM, LRESULT, WPARAM };
use windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U;
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::{ BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT, RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow };
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{ ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent };
use windows::Win32::UI::WindowsAndMessaging::{ CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, GWLP_USERDATA, GetClassNameW, GetForegroundWindow, GetWindowLongPtrW, HWND_TOPMOST, IDC_ARROW, KillTimer, LoadCursorW, MSG, PM_REMOVE, PeekMessageW, PostMessageW, PostQuitMessage, RegisterClassW, SW_HIDE, SW_SHOW, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_APP, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_ERASEBKGND, WM_INPUT, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SETTINGCHANGE, WM_SIZE, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_POPUP };
use windows::core::{ Result as WindowsResult, w };


const WM_START_TOGGLE: u32 = WM_APP + 1;
const WM_START_DISMISS: u32 = WM_APP + 2;
const WM_START_ANIMATION_FRAME: u32 = WM_APP + 3;
const WM_START_TRAY_TOGGLE: u32 = WM_APP + 4;
const WM_START_FOREGROUND_CHANGED: u32 = WM_APP + 6;
const WM_START_UPDATE_PREFERENCES: u32 = WM_APP + 7;
const WM_MOUSELEAVE: u32 = 0x02A3;
const BACKDROP_TIMER_ID: usize = 1;
const TASKBAR_REFOCUS_TIMER_ID: usize = 2;
const BACKDROP_FRAME_INTERVAL_MS: u32 = 33;
const TASKBAR_REFOCUS_DELAY_MS: u32 = 50;


pub struct WindowHost {
	state: *mut WindowState,
}


#[derive( Clone, Copy )]
pub struct StartController {
	hwnd: HWND,
}


struct WindowState {
	hwnd: HWND,
	client_size: D2D_SIZE_U,
	dpi: f32,
	config: StartConfig,
	config_store: ConfigStore,
	preferences: StartPreferences,
	layout: TileLayout,
	renderer: Renderer,
	backdrop: DesktopBackdrop,
	overlay: OverlaySurface,
	transition: DesktopTransition,
	animation: AnimationController,
	animation_frame_pending: bool,
	input: Option< GlobalInputBinding >,
	activation: Option< ForegroundActivation >,
	foreground_observer: Option< ForegroundChangeObserver >,
	hovered_tile: Option< TileAddress >,
	hovered_folder_tile: Option< FolderTileAddress >,
	open_folder: Option< TileAddress >,
	pressed_folder_tile: Option< FolderTileAddress >,
	mouse_tracking: bool,
	drag: Option< PointerDrag >,
}


struct PointerDrag {
	source: DragSource,
	start_x: f32,
	start_y: f32,
	current_x: f32,
	current_y: f32,
	active: bool,
	target: DropTarget,
	preview_source: DragSource,
	origin_rect: windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F,
	original_config: StartConfig,
	preview_config: Option< StartConfig >,
	preview_layout: Option< TileLayout >,
}


impl WindowHost {
	pub fn create( config_store: ConfigStore, config: StartConfig, preferences: StartPreferences, renderer: Renderer, input_manager: &GlobalInputManager ) -> Result< Self, String > {
		let backdrop = DesktopBackdrop::create()?;
		let overlay = OverlaySurface::create()?;
		let transition = DesktopTransition::create()?;
		let state = Box::new( WindowState { hwnd: HWND::default(), client_size: D2D_SIZE_U::default(), dpi: 96.0, config, config_store, preferences, layout: TileLayout::default(), renderer, backdrop, overlay, transition, animation: AnimationController::new( preferences.opening_duration_ms ), animation_frame_pending: false, input: None, activation: None, foreground_observer: None, hovered_tile: None, hovered_folder_tile: None, open_folder: None, pressed_folder_tile: None, mouse_tracking: false, drag: None } );
		let state_pointer = Box::into_raw( state );
		if let Err( error ) = unsafe { Self::create_native_window( state_pointer ) } {
			unsafe { drop( Box::from_raw( state_pointer ) ); }
			return Err( format!( "创建 Start 内容窗口失败：{}", error ) );
		}
		let host = Self { state: state_pointer };
		unsafe { ( *host.state ).prepare_layout(); }
		let hwnd = unsafe { ( *state_pointer ).hwnd };
		let binding = input_manager.bind_start_surface( hwnd, WM_START_TOGGLE, WM_START_DISMISS )?;
		binding.set_shortcut( input_shortcut( preferences.shortcut ) );
		let foreground_observer = ForegroundChangeObserver::watch( hwnd, WM_START_FOREGROUND_CHANGED )?;
		unsafe {
			( *host.state ).input = Some( binding );
			( *host.state ).foreground_observer = Some( foreground_observer );
		}
		Ok( host )
	}


	pub fn controller( &self ) -> StartController {
		StartController { hwnd: unsafe { ( *self.state ).hwnd } }
	}


	unsafe fn create_native_window( state: *mut WindowState ) -> WindowsResult< () > {
		let module = unsafe { GetModuleHandleW( None )? };
		let instance = HINSTANCE( module.0 );
		let class = WNDCLASSW { lpfnWndProc: Some( window_proc ), hInstance: instance, lpszClassName: w!( "Windows.UI.EpStartWindow" ), hCursor: unsafe { LoadCursorW( None, IDC_ARROW )? }, ..Default::default() };
		if unsafe { RegisterClassW( &class ) } == 0 { return Err( windows::core::Error::from_thread() ); }
		unsafe { CreateWindowExW( WS_EX_LAYERED | WS_EX_TOOLWINDOW, w!( "Windows.UI.EpStartWindow" ), w!( "ep_start" ), WS_POPUP, 0, 0, 0, 0, None, None, Some( instance ), Some( state.cast::< c_void >() ) )?; }
		Ok( () )
	}
}


impl StartController {
	pub fn toggle( &self ) {
		self.post_command( WM_START_TOGGLE );
	}


	pub fn toggle_from_tray( &self ) {
		self.post_command( WM_START_TRAY_TOGGLE );
	}


	pub fn update_preferences( &self, preferences: StartPreferences ) {
		if self.hwnd.is_invalid() { return; }
		let pointer = Box::into_raw( Box::new( preferences ) );
		if unsafe { PostMessageW( Some( self.hwnd ), WM_START_UPDATE_PREFERENCES, WPARAM( 0 ), LPARAM( pointer as isize ) ) }.is_err() { unsafe { drop( Box::from_raw( pointer ) ); } }
	}


	fn post_command( &self, message: u32 ) {
		if self.hwnd.is_invalid() { return; }
		unsafe { let _ = PostMessageW( Some( self.hwnd ), message, WPARAM( 0 ), LPARAM( 0 ) ); }
	}
}


impl Drop for WindowHost {
	fn drop( &mut self ) {
		unsafe {
			let state = &mut *self.state;
			drop( state.input.take() );
			drop( state.foreground_observer.take() );
			drop( state.activation.take() );
			state.backdrop.hide();
			state.overlay.hide();
			state.transition.hide();
			if !state.hwnd.is_invalid() { let _ = DestroyWindow( state.hwnd ); }
			drop( Box::from_raw( self.state ) );
		}
	}
}


impl WindowState {
	fn prepare_layout( &mut self ) {
		let Ok( geometry ) = MonitorGeometry::from_cursor() else { return; };
		unsafe { let _ = SetWindowPos( self.hwnd, None, geometry.work_rect.left, geometry.work_rect.top, geometry.work_width(), geometry.work_height(), SWP_NOACTIVATE ); }
		self.apply_geometry( &geometry );
	}


	fn toggle( &mut self ) {
		match self.animation.state() {
			VisibilityState::Hidden => self.begin_open(),
			VisibilityState::Opening | VisibilityState::Visible => self.begin_close(),
			VisibilityState::Closing => {
				self.animation.open();
				self.request_animation_frame();
			}
		}
	}


	fn toggle_from_tray( &mut self ) {
		match self.animation.state() {
			VisibilityState::Hidden => self.begin_open(),
			VisibilityState::Opening | VisibilityState::Visible => self.begin_close(),
			VisibilityState::Closing => { self.animation.open(); self.request_animation_frame(); }
		}
	}


	fn begin_open( &mut self ) {
		unsafe { let _ = KillTimer( Some( self.hwnd ), TASKBAR_REFOCUS_TIMER_ID ); }
		let geometry = MonitorGeometry::from_cursor().or_else( |_| MonitorGeometry::from_window( self.hwnd ) );
		let Ok( geometry ) = geometry else { return; };
		let transition_ready = self.transition.capture( &geometry ).is_ok();
		if transition_ready { self.transition.show( &geometry ); }
		unsafe { let _ = SetWindowPos( self.hwnd, None, geometry.work_rect.left, geometry.work_rect.top, geometry.work_width(), geometry.work_height(), SWP_NOACTIVATE ); }
		self.apply_geometry( &geometry );
		let _ = self.renderer.prepare( self.hwnd, self.client_size, self.dpi );
		self.backdrop.show( &geometry, self.transition.cover_window(), self.preferences.blur_percent );
		self.overlay.show( &geometry, None );
		self.animation.open();
		self.animation.prime_open_frame( 1.0 / 60.0 );
		self.apply_animation_frame();
		unsafe {
			let _ = SetWindowPos( self.hwnd, Some( HWND_TOPMOST ), geometry.work_rect.left, geometry.work_rect.top, geometry.work_width(), geometry.work_height(), SWP_NOACTIVATE | SWP_SHOWWINDOW );
			let _ = ShowWindow( self.hwnd, SW_SHOW );
		}
		self.activation = Some( ForegroundActivation::activate( self.hwnd ) );
		if let Some( observer ) = &mut self.foreground_observer { observer.set_enabled( true ); }
		if let Some( input ) = &self.input { input.set_surface_visible( true ); }
		self.update_backdrop_timer();
		unsafe { let _ = RedrawWindow( Some( self.hwnd ), None, None, RDW_INVALIDATE | RDW_UPDATENOW ); }
		self.animation.synchronize_clock();
		self.request_animation_frame();
	}


	fn begin_close( &mut self ) {
		unsafe { let _ = KillTimer( Some( self.hwnd ), TASKBAR_REFOCUS_TIMER_ID ); }
		self.animation.close();
		if self.animation.state() == VisibilityState::Hidden { self.finish_close(); } else { self.request_animation_frame(); }
	}


	fn advance_animation( &mut self ) {
		self.animation_frame_pending = false;
		self.drain_animation_commands();
		self.animation.advance();
		self.apply_animation_frame();
		unsafe { let _ = RedrawWindow( Some( self.hwnd ), None, None, RDW_INVALIDATE | RDW_UPDATENOW ); }
		unsafe { let _ = DwmFlush(); }
		match self.animation.state() {
			VisibilityState::Hidden => self.finish_close(),
			VisibilityState::Visible => {}
			_ => self.request_animation_frame(),
		}
	}


	fn drain_animation_commands( &mut self ) {
		let mut message = MSG::default();
		while unsafe { PeekMessageW( &mut message, Some( self.hwnd ), WM_START_TOGGLE, WM_START_TRAY_TOGGLE, PM_REMOVE ) }.as_bool() {
			match message.message {
				WM_START_TOGGLE | WM_START_TRAY_TOGGLE => self.toggle(),
				WM_START_DISMISS => self.begin_close(),
				WM_START_ANIMATION_FRAME => self.animation_frame_pending = false,
				_ => {}
			}
		}
	}


	fn apply_animation_frame( &mut self ) {
		let frame = self.animation.frame();
		self.transition.set_opacity( frame.transition_opacity() );
		self.overlay.set_opacity( frame.overlay_opacity( self.preferences.overlay_opacity_percent ) );
		unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); }
	}


	fn update_preferences( &mut self, mut preferences: StartPreferences ) {
		preferences.normalize();
		if let Some( input ) = &self.input { input.set_shortcut( input_shortcut( preferences.shortcut ) ); }
		self.preferences = preferences;
		self.animation.set_duration( preferences.opening_duration_ms );
		let logical_width = self.client_size.width as f32 * 96.0 / self.dpi;
		let logical_height = self.client_size.height as f32 * 96.0 / self.dpi;
		self.layout.calculate( logical_width, logical_height, &self.config, &self.preferences, self.open_folder );
		if self.animation.is_surface_present() {
			if let Ok( geometry ) = MonitorGeometry::from_window( self.hwnd ) { self.backdrop.set_blur( &geometry, preferences.blur_percent ); }
		}
		self.update_backdrop_timer();
		if self.animation.is_surface_present() { self.apply_animation_frame(); }
	}


	fn finish_close( &mut self ) {
		if let Some( observer ) = &mut self.foreground_observer { observer.set_enabled( false ); }
		unsafe {
			let _ = KillTimer( Some( self.hwnd ), BACKDROP_TIMER_ID );
			let _ = KillTimer( Some( self.hwnd ), TASKBAR_REFOCUS_TIMER_ID );
		}
		drop( self.activation.take() );
		unsafe { let _ = ShowWindow( self.hwnd, SW_HIDE ); }
		self.backdrop.hide();
		self.overlay.hide();
		self.transition.hide();
		self.renderer.release_device_resources();
		self.hovered_tile = None;
		self.hovered_folder_tile = None;
		self.open_folder = None;
		self.pressed_folder_tile = None;
		self.mouse_tracking = false;
		self.drag = None;
		if let Some( input ) = &self.input { input.set_surface_visible( false ); }
		trim_working_set();
	}


	fn handle_foreground_change( &mut self ) {
		if !matches!( self.animation.state(), VisibilityState::Opening | VisibilityState::Visible ) { return; }
		let foreground = unsafe { GetForegroundWindow() };
		if foreground.is_invalid() || foreground == self.hwnd { return; }
		if is_taskbar_host_window( foreground ) {
			unsafe { let _ = SetTimer( Some( self.hwnd ), TASKBAR_REFOCUS_TIMER_ID, TASKBAR_REFOCUS_DELAY_MS, None ); }
			return;
		}
		if is_taskbar_preview_window( foreground ) { return; }
		unsafe { let _ = KillTimer( Some( self.hwnd ), TASKBAR_REFOCUS_TIMER_ID ); }
		self.begin_close();
	}


	fn confirm_taskbar_interaction( &mut self ) {
		unsafe { let _ = KillTimer( Some( self.hwnd ), TASKBAR_REFOCUS_TIMER_ID ); }
		if !matches!( self.animation.state(), VisibilityState::Opening | VisibilityState::Visible ) { return; }
		let foreground = unsafe { GetForegroundWindow() };
		if is_taskbar_host_window( foreground ) {
			if let Some( activation ) = &self.activation { activation.reactivate(); }
		} else if !foreground.is_invalid() && foreground != self.hwnd && !is_taskbar_preview_window( foreground ) {
			self.begin_close();
		}
	}


	fn apply_geometry( &mut self, geometry: &MonitorGeometry ) {
		let width = geometry.work_width().max( 1 ) as u32;
		let height = geometry.work_height().max( 1 ) as u32;
		self.client_size = D2D_SIZE_U { width, height };
		self.dpi = unsafe { GetDpiForWindow( self.hwnd ) }.max( 96 ) as f32;
		let logical_width = width as f32 * 96.0 / self.dpi;
		let logical_height = height as f32 * 96.0 / self.dpi;
		self.layout.calculate( logical_width, logical_height, &self.config, &self.preferences, self.open_folder );
		self.renderer.resize( self.client_size, self.dpi );
	}


	fn refresh_geometry( &mut self ) {
		if !self.animation.is_surface_present() { return; }
		let Ok( geometry ) = MonitorGeometry::from_window( self.hwnd ) else { return; };
		self.transition.discard();
		self.backdrop.show( &geometry, None, self.preferences.blur_percent );
		self.overlay.show( &geometry, None );
		unsafe { let _ = SetWindowPos( self.hwnd, Some( HWND_TOPMOST ), geometry.work_rect.left, geometry.work_rect.top, geometry.work_width(), geometry.work_height(), SWP_NOACTIVATE | SWP_SHOWWINDOW ); }
		self.apply_geometry( &geometry );
		self.apply_animation_frame();
	}


	fn update_backdrop_timer( &self ) {
		unsafe {
			if self.animation.is_surface_present() && self.preferences.blur_percent > 0 { let _ = SetTimer( Some( self.hwnd ), BACKDROP_TIMER_ID, BACKDROP_FRAME_INTERVAL_MS, None ); } else { let _ = KillTimer( Some( self.hwnd ), BACKDROP_TIMER_ID ); }
		}
	}


	fn update_backdrop_frame( &mut self ) {
		if !self.animation.is_surface_present() || self.preferences.blur_percent == 0 { return; }
		if let Ok( geometry ) = MonitorGeometry::from_window( self.hwnd ) { self.backdrop.update_frame( &geometry ); }
	}


	fn resize( &mut self, width: u32, height: u32 ) {
		if width == 0 || height == 0 { return; }
		self.client_size = D2D_SIZE_U { width, height };
		self.dpi = unsafe { GetDpiForWindow( self.hwnd ) }.max( 96 ) as f32;
		self.layout.calculate( width as f32 * 96.0 / self.dpi, height as f32 * 96.0 / self.dpi, &self.config, &self.preferences, self.open_folder );
		self.renderer.resize( self.client_size, self.dpi );
	}


	fn paint( &mut self ) {
		if self.animation.is_surface_present() && self.client_size.width > 0 && self.client_size.height > 0 {
			let active_drag = self.drag.as_ref().filter( |drag| drag.active );
			let display_config = active_drag.and_then( |drag| drag.preview_config.as_ref() ).unwrap_or( &self.config );
			let display_layout = active_drag.and_then( |drag| drag.preview_layout.as_ref() ).unwrap_or( &self.layout );
			let source_config = active_drag.map( |drag| &drag.original_config );
			let source_layout = active_drag.map( |_| &self.layout );
			let drag = active_drag.map( |drag| DragVisual { source: drag.source, preview_source: drag.preview_source, origin_rect: drag.origin_rect, delta_x: drag.current_x - drag.start_x, delta_y: drag.current_y - drag.start_y, target: drag.target } );
			let _ = self.renderer.paint( self.hwnd, self.client_size, self.dpi, display_config, display_layout, self.hovered_tile, self.hovered_folder_tile, drag, source_config, source_layout, &self.animation.frame() );
		}
	}


	fn mouse_move( &mut self, x: f32, y: f32 ) {
		if !self.mouse_tracking {
			let mut tracking = TRACKMOUSEEVENT { cbSize: size_of::< TRACKMOUSEEVENT >() as u32, dwFlags: TME_LEAVE, hwndTrack: self.hwnd, ..Default::default() };
			if unsafe { TrackMouseEvent( &mut tracking ) }.is_ok() { self.mouse_tracking = true; }
		}
		let scale = self.dpi / 96.0;
		let logical_x = x / scale;
		let logical_y = y / scale;
		if self.drag.is_some() {
			let display_layout = self.drag.as_ref().and_then( |drag| drag.preview_layout.as_ref() ).unwrap_or( &self.layout );
			let source = self.drag.as_ref().unwrap().source;
			let active = self.drag.as_ref().unwrap().active || ( ( logical_x - self.drag.as_ref().unwrap().start_x ).powi( 2 ) + ( logical_y - self.drag.as_ref().unwrap().start_y ).powi( 2 ) ).sqrt() >= 5.0;
			let target = match source {
				DragSource::Tile( _ ) => {
					let bar_index = display_layout.nearest_bar( logical_x, logical_y ).unwrap_or( 0 );
					let tile_count = self.drag.as_ref().unwrap().original_config.bars.get( bar_index ).map( |bar| bar.tiles.len() ).unwrap_or( 0 );
					DropTarget::Tile { bar_index, slot_index: display_layout.tile_slot_index( bar_index, logical_x, logical_y, tile_count ) }
				}
				DragSource::Bar( _ ) => DropTarget::Bar( display_layout.nearest_bar( logical_x, logical_y ).unwrap_or( 0 ) ),
			};
			let changed = {
				let drag = self.drag.as_mut().unwrap();
				drag.current_x = logical_x;
				drag.current_y = logical_y;
				let changed = drag.target != target || drag.active != active;
				drag.active = active;
				drag.target = target;
				changed
			};
			if active && changed { self.rebuild_drag_preview(); }
			if active { unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); } }
			return;
		}
		let hovered_folder = if self.animation.state() == VisibilityState::Visible { self.layout.hit_test_folder_tile( logical_x, logical_y ) } else { None };
		let hovered = if self.animation.state() == VisibilityState::Visible && hovered_folder.is_none() && !self.layout.folder_contains( logical_x, logical_y ) { self.layout.hit_test( logical_x, logical_y ) } else { None };
		if hovered_folder != self.hovered_folder_tile { self.hovered_folder_tile = hovered_folder; unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); } }
		if hovered != self.hovered_tile { self.hovered_tile = hovered; unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); } }
	}


	fn mouse_leave( &mut self ) {
		self.mouse_tracking = false;
		if self.hovered_tile.take().is_some() { unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); } }
		if self.hovered_folder_tile.take().is_some() { unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); } }
	}


	fn pointer_down( &mut self, x: f32, y: f32 ) {
		if self.animation.state() != VisibilityState::Visible { return; }
		let scale = self.dpi / 96.0;
		let logical_x = x / scale;
		let logical_y = y / scale;
		if let Some( address ) = self.layout.hit_test_folder_tile( logical_x, logical_y ) { self.pressed_folder_tile = Some( address ); return; }
		if self.open_folder.is_some() && !self.layout.folder_contains( logical_x, logical_y ) { self.open_folder = None; self.reflow_layout(); unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); } return; }
		let source = if let Some( address ) = self.layout.hit_test( logical_x, logical_y ) { DragSource::Tile( address ) } else if let Some( bar_index ) = self.layout.hit_test_bar_title( logical_x, logical_y ) { DragSource::Bar( bar_index ) } else { return; };
		let target = match source { DragSource::Tile( address ) => DropTarget::Tile { bar_index: address.bar_index, slot_index: address.tile_index }, DragSource::Bar( bar_index ) => DropTarget::Bar( bar_index ) };
		let origin_rect = match source { DragSource::Tile( address ) => self.layout.tile_rect( address ), DragSource::Bar( bar_index ) => self.layout.bar_rect( bar_index ) };
		let Some( origin_rect ) = origin_rect else { return; };
		self.drag = Some( PointerDrag { source, start_x: logical_x, start_y: logical_y, current_x: logical_x, current_y: logical_y, active: false, target, preview_source: source, origin_rect, original_config: self.config.clone(), preview_config: None, preview_layout: None } );
		unsafe { SetCapture( self.hwnd ); }
	}


	fn pointer_up( &mut self, x: f32, y: f32 ) {
		let scale = self.dpi / 96.0;
		let logical_x = x / scale;
		let logical_y = y / scale;
		if let Some( pressed ) = self.pressed_folder_tile.take() {
			if self.layout.hit_test_folder_tile( logical_x, logical_y ) == Some( pressed ) { self.launch_folder_tile( pressed ); }
			return;
		}
		let Some( drag ) = self.drag.take() else { return; };
		unsafe { let _ = ReleaseCapture(); }
		if !drag.active {
			if let DragSource::Tile( address ) = drag.source {
				if self.config.bars[ address.bar_index ].tiles[ address.tile_index ].is_folder() {
					self.open_folder = if self.open_folder == Some( address ) { None } else { Some( address ) };
					self.reflow_layout();
					unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); }
				} else { self.launch_tile( address ); }
			}
			return;
		}
		if let Some( preview_config ) = drag.preview_config { self.config = preview_config; }
		if let Some( preview_layout ) = drag.preview_layout { self.layout = preview_layout; } else { self.reflow_layout(); }
		self.open_folder = None;
		if let Err( error ) = self.config_store.save( &self.config ) { show_error_dialog( "保存磁贴布局失败", &error ); }
		unsafe { let _ = InvalidateRect( Some( self.hwnd ), None, false ); }
	}


	fn launch_tile( &mut self, address: TileAddress ) {
		let tile = &self.config.bars[ address.bar_index ].tiles[ address.tile_index ];
		if ProgramLauncher::launch( self.hwnd, tile ).is_ok() {
			if let Some( activation ) = &mut self.activation { activation.abandon_restore(); }
			self.transition.discard();
			self.begin_close();
		}
	}


	fn launch_folder_tile( &mut self, address: FolderTileAddress ) {
		let tile = &self.config.bars[ address.owner.bar_index ].tiles[ address.owner.tile_index ].tiles[ address.tile_index ];
		if tile.is_folder() { return; }
		if ProgramLauncher::launch( self.hwnd, tile ).is_ok() {
			if let Some( activation ) = &mut self.activation { activation.abandon_restore(); }
			self.transition.discard();
			self.begin_close();
		}
	}


	fn rebuild_drag_preview( &mut self ) {
		let Some( drag ) = &self.drag else { return; };
		let mut preview = drag.original_config.clone();
		let preview_source = match ( drag.source, drag.target ) {
			( DragSource::Tile( source ), DropTarget::Tile { bar_index, slot_index } ) => DragSource::Tile( move_tile_in_config( &mut preview, source, bar_index, slot_index ) ),
			( DragSource::Bar( source ), DropTarget::Bar( target ) ) => DragSource::Bar( move_bar_in_config( &mut preview, source, target ) ),
			_ => drag.source,
		};
		let logical_width = self.client_size.width as f32 * 96.0 / self.dpi;
		let logical_height = self.client_size.height as f32 * 96.0 / self.dpi;
		let mut preview_layout = TileLayout::default();
		preview_layout.calculate( logical_width, logical_height, &preview, &self.preferences, None );
		if let Some( drag ) = &mut self.drag {
			drag.preview_source = preview_source;
			drag.preview_config = Some( preview );
			drag.preview_layout = Some( preview_layout );
		}
	}


	fn reflow_layout( &mut self ) {
		let logical_width = self.client_size.width as f32 * 96.0 / self.dpi;
		let logical_height = self.client_size.height as f32 * 96.0 / self.dpi;
		self.layout.calculate( logical_width, logical_height, &self.config, &self.preferences, self.open_folder );
	}


	fn request_animation_frame( &mut self ) {
		if !self.animation.is_animating() || self.animation_frame_pending { return; }
		if unsafe { PostMessageW( Some( self.hwnd ), WM_START_ANIMATION_FRAME, WPARAM( 0 ), LPARAM( 0 ) ) }.is_ok() { self.animation_frame_pending = true; }
	}
}


fn move_tile_in_config( config: &mut StartConfig, source: TileAddress, target_bar: usize, mut target_slot: usize ) -> TileAddress {
	if source.bar_index >= config.bars.len() || target_bar >= config.bars.len() || source.tile_index >= config.bars[ source.bar_index ].tiles.len() { return source; }
	let tile = config.bars[ source.bar_index ].tiles.remove( source.tile_index );
	if source.bar_index == target_bar && source.tile_index < target_slot { target_slot = target_slot.saturating_sub( 1 ); }
	target_slot = target_slot.min( config.bars[ target_bar ].tiles.len() );
	config.bars[ target_bar ].tiles.insert( target_slot, tile );
	TileAddress { bar_index: target_bar, tile_index: target_slot }
}


fn move_bar_in_config( config: &mut StartConfig, source: usize, target: usize ) -> usize {
	if source >= config.bars.len() || target >= config.bars.len() || source == target { return source; }
	let bar = config.bars.remove( source );
	let target = target.min( config.bars.len() );
	config.bars.insert( target, bar );
	target
}


unsafe extern "system" fn window_proc( hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM ) -> LRESULT {
	if message == WM_NCCREATE {
		let creation = unsafe { &*( lparam.0 as *const CREATESTRUCTW ) };
		let state = creation.lpCreateParams.cast::< WindowState >();
		unsafe {
			( *state ).hwnd = hwnd;
			SetWindowLongPtrW( hwnd, GWLP_USERDATA, state as isize );
		}
	}
	let state = unsafe { GetWindowLongPtrW( hwnd, GWLP_USERDATA ) as *mut WindowState };
	if !state.is_null() {
		match message {
			WM_INPUT => {
				let action = unsafe { ( *state ).input.as_ref().and_then( |input| input.raw_input_action( lparam ) ) };
				match action {
					Some( GlobalInputAction::Toggle ) => unsafe { ( *state ).toggle() },
					Some( GlobalInputAction::Dismiss ) => unsafe { ( *state ).begin_close() },
					None => {}
				}
				return unsafe { DefWindowProcW( hwnd, message, wparam, lparam ) };
			}
			WM_START_TOGGLE => { unsafe { ( *state ).toggle(); } return LRESULT( 0 ); }
			WM_START_DISMISS => { unsafe { ( *state ).begin_close(); } return LRESULT( 0 ); }
			WM_START_ANIMATION_FRAME => { unsafe { ( *state ).advance_animation(); } return LRESULT( 0 ); }
			WM_START_TRAY_TOGGLE => { unsafe { ( *state ).toggle_from_tray(); } return LRESULT( 0 ); }
			WM_START_FOREGROUND_CHANGED => { unsafe { ( *state ).handle_foreground_change(); } return LRESULT( 0 ); }
			WM_START_UPDATE_PREFERENCES => {
				let preferences = unsafe { *Box::from_raw( lparam.0 as *mut StartPreferences ) };
				unsafe { ( *state ).update_preferences( preferences ); }
				return LRESULT( 0 );
			}
			WM_TIMER => {
				if wparam.0 == BACKDROP_TIMER_ID { unsafe { ( *state ).update_backdrop_frame(); } return LRESULT( 0 ); }
				if wparam.0 == TASKBAR_REFOCUS_TIMER_ID { unsafe { ( *state ).confirm_taskbar_interaction(); } return LRESULT( 0 ); }
			}
			WM_LBUTTONDOWN => {
				let x = lparam.0 as i16 as f32;
				let y = ( lparam.0 >> 16 ) as i16 as f32;
				unsafe { ( *state ).pointer_down( x, y ); }
				return LRESULT( 0 );
			}
			WM_LBUTTONUP => {
				let x = lparam.0 as i16 as f32;
				let y = ( lparam.0 >> 16 ) as i16 as f32;
				unsafe { ( *state ).pointer_up( x, y ); }
				return LRESULT( 0 );
			}
			WM_MOUSEMOVE => {
				let x = lparam.0 as i16 as f32;
				let y = ( lparam.0 >> 16 ) as i16 as f32;
				unsafe { ( *state ).mouse_move( x, y ); }
				return LRESULT( 0 );
			}
			WM_MOUSELEAVE => { unsafe { ( *state ).mouse_leave(); } return LRESULT( 0 ); }
			WM_SIZE => {
				let width = ( lparam.0 & 0xFFFF ) as u32;
				let height = ( ( lparam.0 >> 16 ) & 0xFFFF ) as u32;
				unsafe { ( *state ).resize( width, height ); }
				return LRESULT( 0 );
			}
			WM_DISPLAYCHANGE | WM_SETTINGCHANGE | WM_DPICHANGED => { unsafe { ( *state ).refresh_geometry(); } return LRESULT( 0 ); }
			WM_PAINT => {
				let mut paint = PAINTSTRUCT::default();
				unsafe {
					BeginPaint( hwnd, &mut paint );
					( *state ).paint();
					let _ = EndPaint( hwnd, &paint );
				}
				return LRESULT( 0 );
			}
			WM_ERASEBKGND => { return LRESULT( 1 ); }
			WM_DESTROY => { unsafe { PostQuitMessage( 0 ); } return LRESULT( 0 ); }
			WM_NCDESTROY => { unsafe { SetWindowLongPtrW( hwnd, GWLP_USERDATA, 0 ); } }
			_ => {}
		}
	}
	unsafe { DefWindowProcW( hwnd, message, wparam, lparam ) }
}


fn window_class_name( hwnd: HWND ) -> String {
	let mut class_name = [ 0u16; 128 ];
	let length = unsafe { GetClassNameW( hwnd, &mut class_name ) };
	if length <= 0 { return String::new(); }
	String::from_utf16_lossy( &class_name[ ..length as usize ] )
}


fn is_taskbar_host_window( hwnd: HWND ) -> bool {
	matches!( window_class_name( hwnd ).as_str(), "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" )
}


fn is_taskbar_preview_window( hwnd: HWND ) -> bool {
	matches!( window_class_name( hwnd ).as_str(), "TaskListThumbnailWnd" | "TaskListThumbnailWndXaml" )
}


fn input_shortcut( shortcut: StartShortcut ) -> GlobalStartShortcut {
	match shortcut {
		StartShortcut::WinShift => GlobalStartShortcut::WinShift,
		StartShortcut::Win => GlobalStartShortcut::Win,
	}
}


#[cfg( test )]
mod tests {
	use super::*;
	use crate::config::{ Tile, TileBar };


	#[test]
	fn tile_can_move_between_bars_and_reflows_both() {
		let mut config = StartConfig { bars: vec![ bar( "left", &[ "a", "b", "c" ] ), bar( "right", &[ "d", "e" ] ) ] };
		let address = move_tile_in_config( &mut config, TileAddress { bar_index: 0, tile_index: 1 }, 1, 1 );
		assert_eq!( address, TileAddress { bar_index: 1, tile_index: 1 } );
		assert_eq!( titles( &config.bars[ 0 ] ), vec![ "a", "c" ] );
		assert_eq!( titles( &config.bars[ 1 ] ), vec![ "d", "b", "e" ] );
	}


	fn bar( title: &str, values: &[ &str ] ) -> TileBar {
		TileBar { title: title.to_string(), tiles: values.iter().map( |value| Tile { title: ( *value ).to_string(), target: "test.exe".to_string(), arguments: String::new(), working_directory: String::new(), color: "#0067C0".to_string(), tiles: Vec::new() } ).collect() }
	}


	fn titles( bar: &TileBar ) -> Vec< &str > {
		bar.tiles.iter().map( |tile| tile.title.as_str() ).collect()
	}
}
