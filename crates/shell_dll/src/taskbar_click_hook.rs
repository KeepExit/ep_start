//! ::  Project Path  ->  ep_start :: taskbar_click_hook.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/22 15:41 周一


use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{ AtomicBool, AtomicU64, AtomicUsize, Ordering };
use std::thread;
use std::time::Instant;
use windows::Win32::Foundation::{ HWND, LPARAM, POINT, RECT };
use windows::Win32::System::Com::{ CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize };
use windows::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{ MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualAlloc, VirtualProtect };
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{ CUIAutomation, IUIAutomation, IUIAutomationTogglePattern, TreeScope_Descendants, UIA_AutomationIdPropertyId, UIA_TogglePatternId };
use windows::Win32::UI::WindowsAndMessaging::{ EnumChildWindows, FindWindowW, GA_ROOT, GetAncestor, GetClassNameW, GetWindowRect };
use windows::core::{ GUID, HRESULT, IInspectable, IInspectable_Vtbl, Interface, PCWSTR, w };


const IMAGE_DOS_SIGNATURE: u16 = 0x5A4D;
const IMAGE_NT_SIGNATURE: u32 = 0x00004550;
const IMAGE_SCN_MEM_EXECUTE: u32 = 0x20000000;
const SECTION_HEADER_SIZE: usize = 40;
const HOOK_SIZE: usize = 14;
const ABSOLUTE_JUMP_SIZE: usize = 14;
const START_CLICK_PATTERN: [ u8; 48 ] = [ 0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x18, 0x55, 0x57, 0x41, 0x56, 0x48, 0x8B, 0xEC, 0x48, 0x81, 0xEC, 0xCC, 0xCC, 0xCC, 0xCC, 0x48, 0x8B, 0x05, 0xCC, 0xCC, 0xCC, 0xCC, 0x48, 0x33, 0xC4, 0x48, 0x89, 0x45, 0xF0, 0x48, 0x8B, 0xDA, 0x48, 0x8D, 0x55, 0xE0, 0x48, 0x8B, 0xCB ];
const START_CLICK_PROLOGUE: [ u8; HOOK_SIZE ] = [ 0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x18, 0x55, 0x57, 0x41, 0x56 ];
const START_CLICK_SUPPRESSION_WINDOW_MS: u64 = 500;

static START_POINTER_CLOCK: OnceLock< Instant > = OnceLock::new();
static START_POINTER_PENDING: AtomicBool = AtomicBool::new( false );
static START_CLICK_SUPPRESSION_DEADLINE_MS: AtomicU64 = AtomicU64::new( 0 );
static START_EXPERIENCE_VISIBLE: AtomicBool = AtomicBool::new( false );
static START_BUTTON_BOOTSTRAP_PENDING: AtomicBool = AtomicBool::new( false );
static START_BUTTON_BOOTSTRAP_RUNNING: AtomicBool = AtomicBool::new( false );
static ORIGINAL_CLICK_HANDLER: AtomicUsize = AtomicUsize::new( 0 );
type ExperienceToggleButtonClick = unsafe extern "system" fn( *mut c_void, *const c_void, *const c_void );


thread_local! {
	static START_BUTTON: RefCell< Option< ExperienceToggleButton > > = const { RefCell::new( None ) };
}


#[repr( transparent )]
#[derive( Clone )]
struct ExperienceToggleButton( IInspectable );


#[repr( C )]
struct ExperienceToggleButtonVtable {
	base: IInspectable_Vtbl,
	get_is_experience_visible: unsafe extern "system" fn( *mut c_void, *mut bool ) -> HRESULT,
	set_is_experience_visible: unsafe extern "system" fn( *mut c_void, bool ) -> HRESULT,
}


unsafe impl Interface for ExperienceToggleButton {
	type Vtable = ExperienceToggleButtonVtable;
	const IID: GUID = GUID::from_u128( 0x47276054_2ca3_520f_9300_49be3f0aa93c );
}


pub( crate ) fn install() -> bool {
	if ORIGINAL_CLICK_HANDLER.load( Ordering::SeqCst ) != 0 { return true; }
	unsafe { install_inner() }
}


pub( crate ) fn note_pointer_down( target: HWND, point: POINT ) {
	START_POINTER_PENDING.store( unsafe { is_start_button_point( target, point ) }, Ordering::SeqCst );
}


pub( crate ) fn handle_pointer_up( target: HWND, point: POINT ) -> bool {
	let pressed = START_POINTER_PENDING.swap( false, Ordering::SeqCst );
	if !pressed || !unsafe { is_start_button_point( target, point ) } { return false; }
	if !crate::route_start_action( crate::START_ACTION_BUTTON_CLICK ) { return false; }
	START_CLICK_SUPPRESSION_DEADLINE_MS.store( pointer_clock_ms().saturating_add( START_CLICK_SUPPRESSION_WINDOW_MS ), Ordering::SeqCst );
	true
}


pub( crate ) fn set_experience_visible( visible: bool ) {
	START_EXPERIENCE_VISIBLE.store( visible, Ordering::SeqCst );
	let button = START_BUTTON.with( |button| button.borrow().clone() );
	if let Some( button ) = button {
		unsafe { let _ = set_button_experience_visible( &button, visible ); }
	} else if visible && START_CLICK_SUPPRESSION_DEADLINE_MS.load( Ordering::SeqCst ) == 0 {
		request_start_button_bootstrap();
	}
}


pub( crate ) fn is_taskbar_point( point: POINT ) -> bool {
	let mut search = TaskbarPointSearch { point, found: false };
	unsafe { let _ = windows::Win32::UI::WindowsAndMessaging::EnumWindows( Some( find_taskbar_point_callback ), LPARAM( ( &mut search as *mut TaskbarPointSearch ) as isize ) ); }
	search.found
}


unsafe fn install_inner() -> bool {
	let Ok( module ) = ( unsafe { GetModuleHandleW( w!( "Taskbar.View.dll" ) ) } ) else { return false; };
	let base = module.0 as *mut u8;
	let Some( target ) = ( unsafe { find_click_handler( base ) } ) else { return false; };
	if unsafe { std::slice::from_raw_parts( target, HOOK_SIZE ) } != START_CLICK_PROLOGUE { return false; }
	let trampoline_size = HOOK_SIZE + ABSOLUTE_JUMP_SIZE;
	let trampoline = unsafe { VirtualAlloc( None, trampoline_size, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE ) } as *mut u8;
	if trampoline.is_null() { return false; }
	unsafe {
		std::ptr::copy_nonoverlapping( target, trampoline, HOOK_SIZE );
		write_absolute_jump( trampoline.add( HOOK_SIZE ), target.add( HOOK_SIZE ) );
	}
	ORIGINAL_CLICK_HANDLER.store( trampoline as usize, Ordering::SeqCst );
	if !unsafe { patch_target( target, experience_toggle_button_click_hook as *const () as *const u8 ) } {
		ORIGINAL_CLICK_HANDLER.store( 0, Ordering::SeqCst );
		return false;
	}
	true
}


unsafe fn find_click_handler( base: *mut u8 ) -> Option< *mut u8 > {
	if base.is_null() || unsafe { read_u16( base, 0 ) } != IMAGE_DOS_SIGNATURE { return None; }
	let nt_offset = unsafe { read_u32( base, 0x3C ) } as usize;
	if unsafe { read_u32( base, nt_offset ) } != IMAGE_NT_SIGNATURE { return None; }
	let section_count = unsafe { read_u16( base, nt_offset + 6 ) } as usize;
	let optional_size = unsafe { read_u16( base, nt_offset + 20 ) } as usize;
	let sections = nt_offset + 24 + optional_size;
	let mut found = None;
	for index in 0..section_count {
		let section = sections + index * SECTION_HEADER_SIZE;
		let characteristics = unsafe { read_u32( base, section + 36 ) };
		if characteristics & IMAGE_SCN_MEM_EXECUTE == 0 { continue; }
		let size = unsafe { read_u32( base, section + 8 ) } as usize;
		let address = unsafe { read_u32( base, section + 12 ) } as usize;
		let bytes = unsafe { std::slice::from_raw_parts( base.add( address ), size ) };
		for offset in 0..=bytes.len().saturating_sub( START_CLICK_PATTERN.len() ) {
			if !pattern_matches( &bytes[ offset..offset + START_CLICK_PATTERN.len() ] ) { continue; }
			if found.is_some() { return None; }
			found = Some( unsafe { base.add( address + offset ) } );
		}
	}
	found
}


fn pattern_matches( bytes: &[ u8 ] ) -> bool {
	bytes.iter().zip( START_CLICK_PATTERN ).all( |( value, expected )| expected == 0xCC || *value == expected )
}


unsafe fn patch_target( target: *mut u8, replacement: *const u8 ) -> bool {
	let mut previous = PAGE_PROTECTION_FLAGS::default();
	if unsafe { VirtualProtect( target.cast(), HOOK_SIZE, PAGE_EXECUTE_READWRITE, &mut previous ) }.is_err() { return false; }
	unsafe { write_absolute_jump( target, replacement ); }
	unsafe { let _ = FlushInstructionCache( GetCurrentProcess(), Some( target.cast() ), HOOK_SIZE ); }
	let mut ignored = PAGE_PROTECTION_FLAGS::default();
	unsafe { let _ = VirtualProtect( target.cast(), HOOK_SIZE, previous, &mut ignored ); }
	true
}


unsafe fn write_absolute_jump( target: *mut u8, destination: *const u8 ) {
	unsafe {
		target.write( 0xFF );
		target.add( 1 ).write( 0x25 );
		std::ptr::write_bytes( target.add( 2 ), 0, 4 );
		std::ptr::write_unaligned( target.add( 6 ).cast::< usize >(), destination as usize );
	}
}


unsafe extern "system" fn experience_toggle_button_click_hook( this: *mut c_void, sender: *const c_void, arguments: *const c_void ) {
	if START_BUTTON_BOOTSTRAP_PENDING.swap( false, Ordering::SeqCst ) {
		unsafe { capture_start_button( sender ); }
		return;
	}
	let deadline = START_CLICK_SUPPRESSION_DEADLINE_MS.swap( 0, Ordering::SeqCst );
	let pending = deadline != 0 && pointer_clock_ms() <= deadline;
	if pending {
		unsafe { capture_start_button( sender ); }
		return;
	}
	let original = ORIGINAL_CLICK_HANDLER.load( Ordering::SeqCst );
	if original == 0 { return; }
	let original: ExperienceToggleButtonClick = unsafe { std::mem::transmute( original ) };
	unsafe { original( this, sender, arguments ); }
}


fn request_start_button_bootstrap() {
	if START_BUTTON_BOOTSTRAP_RUNNING.swap( true, Ordering::SeqCst ) { return; }
	START_BUTTON_BOOTSTRAP_PENDING.store( true, Ordering::SeqCst );
	thread::spawn( || {
		let initialized = unsafe { CoInitializeEx( None, COINIT_MULTITHREADED ) }.is_ok();
		if initialized {
			let _ = bootstrap_start_button();
			unsafe { CoUninitialize(); }
		}
		START_BUTTON_BOOTSTRAP_PENDING.store( false, Ordering::SeqCst );
		START_BUTTON_BOOTSTRAP_RUNNING.store( false, Ordering::SeqCst );
	} );
}


fn bootstrap_start_button() -> windows::core::Result< () > {
	unsafe {
		let automation: IUIAutomation = CoCreateInstance( &CUIAutomation, None, CLSCTX_INPROC_SERVER )?;
		let taskbar = FindWindowW( w!( "Shell_TrayWnd" ), PCWSTR::null() )?;
		let root = automation.ElementFromHandle( taskbar )?;
		let value = VARIANT::from( "StartButton" );
		let condition = automation.CreatePropertyCondition( UIA_AutomationIdPropertyId, &value )?;
		let button = root.FindFirst( TreeScope_Descendants, &condition )?;
		let toggle: IUIAutomationTogglePattern = button.GetCurrentPatternAs( UIA_TogglePatternId )?;
		toggle.Toggle()
	}
}


unsafe fn capture_start_button( sender: *const c_void ) {
	if sender.is_null() { return; }
	let raw_sender = unsafe { *sender.cast::< *mut c_void >() };
	let Some( inspectable ) = ( unsafe { IInspectable::from_raw_borrowed( &raw_sender ) } ) else { return; };
	let Ok( button ) = inspectable.cast::< ExperienceToggleButton >() else { return; };
	unsafe { let _ = set_button_experience_visible( &button, START_EXPERIENCE_VISIBLE.load( Ordering::SeqCst ) ); }
	START_BUTTON.with( |cached| *cached.borrow_mut() = Some( button ) );
}


fn pointer_clock_ms() -> u64 {
	START_POINTER_CLOCK.get_or_init( Instant::now ).elapsed().as_millis().min( u64::MAX as u128 ) as u64
}


unsafe fn set_button_experience_visible( button: &ExperienceToggleButton, visible: bool ) -> bool {
	unsafe { ( button.vtable().set_is_experience_visible )( button.as_raw(), visible ).is_ok() }
}


unsafe fn is_start_button_point( target: HWND, point: POINT ) -> bool {
	let taskbar = unsafe { GetAncestor( target, GA_ROOT ) };
	if !taskbar.is_invalid() && is_taskbar_window( taskbar ) && unsafe { point_in_start_button( taskbar, point ) } { return true; }
	let Ok( main_taskbar ) = ( unsafe { FindWindowW( w!( "Shell_TrayWnd" ), PCWSTR::null() ) } ) else { return false; };
	unsafe { point_in_start_button( main_taskbar, point ) }
}


unsafe fn point_in_start_button( taskbar: HWND, point: POINT ) -> bool {
	let mut search = StartButtonSearch { point, found: false, task_list: None };
	unsafe { let _ = EnumChildWindows( Some( taskbar ), Some( find_start_button_callback ), LPARAM( ( &mut search as *mut StartButtonSearch ) as isize ) ); }
	if search.found { return true; }
	let Some( task_list ) = search.task_list else { return false; };
	let mut taskbar_rect = RECT::default();
	if unsafe { GetWindowRect( taskbar, &mut taskbar_rect ) }.is_err() { return false; }
	point_in_rect( point, inferred_start_button_rect( taskbar_rect, task_list ) )
}


struct StartButtonSearch {
	point: POINT,
	found: bool,
	task_list: Option< RECT >,
}


struct TaskbarPointSearch {
	point: POINT,
	found: bool,
}


unsafe extern "system" fn find_start_button_callback( hwnd: HWND, lparam: LPARAM ) -> windows::core::BOOL {
	let search = unsafe { &mut *( lparam.0 as *mut StartButtonSearch ) };
	let class_name = window_class_name( hwnd );
	let mut rect = RECT::default();
	if unsafe { GetWindowRect( hwnd, &mut rect ) }.is_err() { return true.into(); }
	if class_name == "MSTaskListWClass" { search.task_list = Some( rect ); }
	if class_name == "Start" && point_in_rect( search.point, rect ) {
		search.found = true;
		return false.into();
	}
	true.into()
}


unsafe extern "system" fn find_taskbar_point_callback( hwnd: HWND, lparam: LPARAM ) -> windows::core::BOOL {
	let search = unsafe { &mut *( lparam.0 as *mut TaskbarPointSearch ) };
	if !is_taskbar_window( hwnd ) { return true.into(); }
	let mut rect = RECT::default();
	if unsafe { GetWindowRect( hwnd, &mut rect ) }.is_ok() && point_in_rect( search.point, rect ) {
		search.found = true;
		return false.into();
	}
	true.into()
}


fn inferred_start_button_rect( taskbar: RECT, task_list: RECT ) -> RECT {
	let width = taskbar.right - taskbar.left;
	let height = taskbar.bottom - taskbar.top;
	if width >= height {
		let size = height.max( 1 );
		RECT { left: ( task_list.left - size ).max( taskbar.left ), top: taskbar.top, right: task_list.left.min( taskbar.right ), bottom: taskbar.bottom }
	} else {
		let size = width.max( 1 );
		RECT { left: taskbar.left, top: ( task_list.top - size ).max( taskbar.top ), right: taskbar.right, bottom: task_list.top.min( taskbar.bottom ) }
	}
}


fn is_taskbar_window( window: HWND ) -> bool {
	matches!( window_class_name( window ).as_str(), "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" )
}


fn window_class_name( window: HWND ) -> String {
	let mut class_name = [ 0u16; 128 ];
	let length = unsafe { GetClassNameW( window, &mut class_name ) };
	if length <= 0 { return String::new(); }
	String::from_utf16_lossy( &class_name[ ..length as usize ] )
}


fn point_in_rect( point: POINT, rect: RECT ) -> bool {
	point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
}


unsafe fn read_u16( base: *const u8, offset: usize ) -> u16 {
	unsafe { std::ptr::read_unaligned( base.add( offset ).cast::< u16 >() ) }
}


unsafe fn read_u32( base: *const u8, offset: usize ) -> u32 {
	unsafe { std::ptr::read_unaligned( base.add( offset ).cast::< u32 >() ) }
}
