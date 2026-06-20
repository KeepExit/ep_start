//! ::  Project Path  ->  ep_start :: focus.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 05:13 周六


use windows::Win32::Foundation::HWND;
use windows::Win32::System::Threading::AttachThreadInput;
use windows::Win32::UI::Input::KeyboardAndMouse::{ SetActiveWindow, SetFocus };
use windows::Win32::UI::WindowsAndMessaging::{ BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow };


pub struct ForegroundActivation {
	target: HWND,
	previous: Option< HWND >,
	restore_enabled: bool,
}


impl ForegroundActivation {
	pub fn activate( target: HWND ) -> Self {
		let foreground = unsafe { GetForegroundWindow() };
		let previous = ( !foreground.is_invalid() && foreground != target ).then_some( foreground );
		activate_window( target );
		Self { target, previous, restore_enabled: true }
	}


	pub fn abandon_restore( &mut self ) {
		self.restore_enabled = false;
		self.previous = None;
	}


	pub fn restore( &mut self ) {
		if !self.restore_enabled { return; }
		self.restore_enabled = false;
		let Some( previous ) = self.previous.take() else { return; };
		let foreground = unsafe { GetForegroundWindow() };
		if foreground == self.target && unsafe { IsWindow( Some( previous ) ) }.as_bool() { activate_window( previous ); }
	}
}


struct InputQueueAttachment {
	from: u32,
	to: u32,
	attached: bool,
}


impl InputQueueAttachment {
	fn between( from: HWND, to: HWND ) -> Self {
		let from_thread = unsafe { GetWindowThreadProcessId( from, None ) };
		let to_thread = unsafe { GetWindowThreadProcessId( to, None ) };
		let attached = from_thread != 0 && to_thread != 0 && from_thread != to_thread && unsafe { AttachThreadInput( from_thread, to_thread, true ) }.as_bool();
		Self { from: from_thread, to: to_thread, attached }
	}
}


impl Drop for InputQueueAttachment {
	fn drop( &mut self ) {
		if self.attached { unsafe { let _ = AttachThreadInput( self.from, self.to, false ); } }
	}
}


fn activate_window( target: HWND ) {
	if target.is_invalid() { return; }
	let foreground = unsafe { GetForegroundWindow() };
	let _attachment = InputQueueAttachment::between( target, foreground );
	unsafe {
		let _ = BringWindowToTop( target );
		let _ = SetActiveWindow( target );
		let _ = SetForegroundWindow( target );
		let _ = SetFocus( Some( target ) );
	}
}


impl Drop for ForegroundActivation {
	fn drop( &mut self ) {
		self.restore();
	}
}
