//! ::  Project Path  ->  ep_start :: event.rs :: event
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 14:38 周日


use crate::state::{ PointerDrag, PointerUpAction, SettingsState };
use crate::host::{ dpi_for_window, request_repaint };
use crate::ui::components::choice_control_contains;
use crate::ui::settings::{ ActionId, ControlKind, SettingId };


impl SettingsState {
	pub( crate ) fn hit_test_slider( &self, x: i32, y: i32 ) -> Option< SettingId > {
		let layout = self.layout();
		let id = layout.hit_control( x, y )?;
		layout.item( id ).filter( |item| item.control_kind == ControlKind::Slider ).map( |item| item.id )
	}
	pub( crate ) fn hit_test_choice( &self, x: i32, y: i32 ) -> Option< SettingId > {
		let layout = self.layout();
		let id = layout.hit_control( x, y )?;
		layout.item( id ).filter( |item| item.control_kind == ControlKind::Choice && choice_control_contains( item.control, dpi_for_window( self.hwnd ), x, y ) ).map( |item| item.id )
	}
	pub( crate ) fn scroll_to( &mut self, position: i32 ) {
		let maximum = self.layout().content.scroll_max;
		let position = position.clamp( 0, maximum );
		if position == self.scroll_y { return; }
		self.scroll_y = position;
		request_repaint( self.hwnd );
	}
	pub( crate ) fn on_pointer_down( &mut self, x: i32, y: i32 ) -> bool {
		if self.begin_scroll_drag( x, y ) {
			return true;
		}
		if let Some( field ) = self.hit_test_slider( x, y ) {
			self.pointer_drag = Some( PointerDrag::Slider( field ) );
			self.update_slider( field, x );
			return true;
		}
		false
	}
	pub( crate ) fn on_pointer_move( &mut self, x: i32, y: i32 ) {
		match self.pointer_drag {
			Some( PointerDrag::Slider( field ) ) => self.update_slider( field, x ),
			Some( PointerDrag::Scrollbar( offset ) ) => self.update_scroll_drag( y, offset ),
			None => {}
		}
	}
	pub( crate ) fn on_pointer_up( &mut self, x: i32, y: i32 ) -> PointerUpAction {
		if self.pointer_drag.take().is_some() {
			request_repaint( self.hwnd );
			return PointerUpAction::ReleaseCapture;
		}
		let layout = self.layout();
		if let Some( action ) = layout.hit_action( x, y ) {
			match action {
				ActionId::Undo => self.undo(),
				ActionId::Save => self.save(),
			}
			return PointerUpAction::None;
		}
		if let Some( field ) = self.hit_test_choice( x, y ) {
			return PointerUpAction::Choice( field );
		}
		PointerUpAction::None
	}
	pub( crate ) fn on_capture_changed( &mut self ) {
		if self.pointer_drag.take().is_some() {
			request_repaint( self.hwnd );
		}
	}
	pub( crate ) fn on_size( &mut self ) {
		let maximum = self.layout().content.scroll_max;
		self.scroll_y = self.scroll_y.clamp( 0, maximum );
		request_repaint( self.hwnd );
	}
	pub( crate ) fn on_mouse_wheel( &mut self, delta: i32 ) {
		let position = self.scroll_y - delta / 120 * 72;
		self.scroll_to( position );
	}
	pub( crate ) fn begin_scroll_drag( &mut self, x: i32, y: i32 ) -> bool {
		let layout = self.layout();
		if layout.hit_scroll_thumb( x, y ) {
			let thumb_top = layout.content.scrollbar.unwrap().thumb.top;
			self.pointer_drag = Some( PointerDrag::Scrollbar( y - thumb_top ) );
			return true;
		}
		if layout.hit_scroll_track( x, y ) {
			let scrollbar = layout.content.scrollbar.unwrap();
			let offset = ( scrollbar.thumb.bottom - scrollbar.thumb.top ) / 2;
			self.scroll_to( layout.scroll_from_thumb( y - offset ) );
			self.pointer_drag = Some( PointerDrag::Scrollbar( offset ) );
			return true;
		}
		false
	}
	pub( crate ) fn update_scroll_drag( &mut self, y: i32, offset: i32 ) {
		let layout = self.layout();
		self.scroll_to( layout.scroll_from_thumb( y - offset ) );
	}
}
