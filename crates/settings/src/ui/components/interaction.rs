//! ::  Project Path  ->  ep_start :: interaction.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/22 03:21 周一


use crate::ui::settings::{ ActionId, SettingId };
use std::time::Instant;


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub( crate ) enum InteractionId {
	Setting( SettingId ),
	Action( ActionId ),
}


#[derive( Clone, Copy, Debug, Default )]
pub( crate ) struct InteractionVisual {
	pub( crate ) hover: f32,
	pub( crate ) press: f32,
	pub( crate ) toggle: f32,
}


pub( crate ) struct InteractionAnimations {
	entries: Vec< InteractionEntry >,
	hovered: Option< InteractionId >,
	pressed: Option< InteractionId >,
	last_tick: Instant,
}


struct InteractionEntry {
	id: InteractionId,
	visual: InteractionVisual,
	toggle_target: f32,
}


impl InteractionAnimations {
	pub( crate ) fn new( switch_enabled: bool ) -> Self {
		let toggle = switch_enabled as u8 as f32;
		Self { entries: vec![ InteractionEntry { id: InteractionId::Setting( SettingId::StartButtonClick ), visual: InteractionVisual { toggle, ..Default::default() }, toggle_target: toggle } ], hovered: None, pressed: None, last_tick: Instant::now() }
	}


	pub( crate ) fn hovered( &self ) -> Option< InteractionId > {
		self.hovered
	}


	pub( crate ) fn pressed( &self ) -> Option< InteractionId > {
		self.pressed
	}


	pub( crate ) fn set_hovered( &mut self, hovered: Option< InteractionId > ) {
		if self.hovered == hovered { return; }
		if let Some( id ) = hovered { self.ensure_entry( id ); }
		self.hovered = hovered;
		self.last_tick = Instant::now();
	}


	pub( crate ) fn set_pressed( &mut self, pressed: Option< InteractionId > ) {
		if self.pressed == pressed { return; }
		if let Some( id ) = pressed { self.ensure_entry( id ); }
		self.pressed = pressed;
		self.last_tick = Instant::now();
	}


	pub( crate ) fn set_toggle( &mut self, id: InteractionId, enabled: bool ) {
		let entry = self.ensure_entry( id );
		entry.toggle_target = enabled as u8 as f32;
		self.last_tick = Instant::now();
	}


	pub( crate ) fn visual( &self, id: InteractionId ) -> InteractionVisual {
		self.entries.iter().find( |entry| entry.id == id ).map( |entry| entry.visual ).unwrap_or_default()
	}


	pub( crate ) fn advance( &mut self ) {
		let now = Instant::now();
		let elapsed = now.duration_since( self.last_tick ).as_secs_f32().min( 0.05 );
		self.last_tick = now;
		for entry in &mut self.entries {
			entry.visual.hover = approach( entry.visual.hover, ( self.hovered == Some( entry.id ) ) as u8 as f32, elapsed, 14.0 );
			entry.visual.press = approach( entry.visual.press, ( self.pressed == Some( entry.id ) && self.hovered == Some( entry.id ) ) as u8 as f32, elapsed, 22.0 );
			entry.visual.toggle = approach( entry.visual.toggle, entry.toggle_target, elapsed, 18.0 );
		}
	}


	pub( crate ) fn is_animating( &self ) -> bool {
		self.entries.iter().any( |entry| !near( entry.visual.hover, ( self.hovered == Some( entry.id ) ) as u8 as f32 ) || !near( entry.visual.press, ( self.pressed == Some( entry.id ) && self.hovered == Some( entry.id ) ) as u8 as f32 ) || !near( entry.visual.toggle, entry.toggle_target ) )
	}


	pub( crate ) fn clear_pointer( &mut self ) {
		self.set_hovered( None );
		self.set_pressed( None );
	}


	fn ensure_entry( &mut self, id: InteractionId ) -> &mut InteractionEntry {
		if let Some( index ) = self.entries.iter().position( |entry| entry.id == id ) { return &mut self.entries[ index ]; }
		self.entries.push( InteractionEntry { id, visual: InteractionVisual::default(), toggle_target: 0.0 } );
		self.entries.last_mut().unwrap()
	}
}


fn approach( current: f32, target: f32, elapsed: f32, speed: f32 ) -> f32 {
	let value = current + ( target - current ) * ( 1.0 - ( -speed * elapsed ).exp() );
	if near( value, target ) { target } else { value }
}


fn near( value: f32, target: f32 ) -> bool {
	( value - target ).abs() < 0.002
}
