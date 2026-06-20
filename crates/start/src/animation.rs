//! ::  Project Path  ->  ep_start :: animation.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 01:19 周日


use std::time::Instant;


const TILE_DELAY_NORMALIZED: f32 = 0.028;
const MAX_STAGGER_INDEX: usize = 12;


#[derive( Clone, Copy, Debug, Eq, PartialEq )]
pub enum VisibilityState {
	Hidden,
	Opening,
	Visible,
	Closing,
}


#[derive( Clone, Copy, Debug )]
pub struct AnimationFrame {
	progress: f32,
}


pub struct AnimationController {
	progress: f32,
	segment_start: f32,
	target: f32,
	segment_elapsed: f32,
	segment_duration: f32,
	last_tick: Instant,
	duration_seconds: f32,
}


impl AnimationController {
	pub fn new( duration_ms: u32 ) -> Self {
		Self { progress: 0.0, segment_start: 0.0, target: 0.0, segment_elapsed: 0.0, segment_duration: 0.0, last_tick: Instant::now(), duration_seconds: duration_ms as f32 / 1000.0 }
	}


	pub fn state( &self ) -> VisibilityState {
		if self.target >= 0.5 {
			if self.progress >= 1.0 { VisibilityState::Visible } else { VisibilityState::Opening }
		} else if self.progress <= 0.0 { VisibilityState::Hidden } else { VisibilityState::Closing }
	}


	pub fn frame( &self ) -> AnimationFrame {
		AnimationFrame { progress: self.progress.clamp( 0.0, 1.0 ) }
	}


	pub fn open( &mut self ) {
		self.set_target( 1.0 );
	}


	pub fn close( &mut self ) {
		self.set_target( 0.0 );
	}


	pub fn set_duration( &mut self, duration_ms: u32 ) {
		self.duration_seconds = duration_ms as f32 / 1000.0;
		if self.is_animating() { self.start_segment( self.target ); }
	}


	pub fn advance( &mut self ) {
		let now = Instant::now();
		let elapsed = now.duration_since( self.last_tick ).as_secs_f32().min( 0.1 );
		self.last_tick = now;
		self.advance_by( elapsed );
	}


	pub fn prime_open_frame( &mut self, frame_seconds: f32 ) {
		if self.target >= 0.5 && self.progress == 0.0 { self.advance_by( frame_seconds.max( 0.0 ) ); self.last_tick = Instant::now(); }
	}


	pub fn synchronize_clock( &mut self ) {
		self.last_tick = Instant::now();
	}


	pub fn is_animating( &self ) -> bool {
		matches!( self.state(), VisibilityState::Opening | VisibilityState::Closing )
	}


	pub fn is_surface_present( &self ) -> bool {
		self.state() != VisibilityState::Hidden
	}


	fn set_target( &mut self, target: f32 ) {
		if self.target == target { return; }
		self.start_segment( target );
	}


	fn start_segment( &mut self, target: f32 ) {
		self.segment_start = self.progress;
		self.target = target;
		self.segment_elapsed = 0.0;
		self.segment_duration = self.duration_seconds * ( target - self.progress ).abs();
		self.last_tick = Instant::now();
		if self.segment_duration <= f32::EPSILON { self.progress = target; }
	}


	fn advance_by( &mut self, elapsed: f32 ) {
		if elapsed <= 0.0 || !self.is_animating() { return; }
		self.segment_elapsed = ( self.segment_elapsed + elapsed ).min( self.segment_duration );
		let linear = if self.segment_duration <= f32::EPSILON { 1.0 } else { self.segment_elapsed / self.segment_duration };
		let eased = ease_out_cubic( linear.clamp( 0.0, 1.0 ) );
		self.progress = self.segment_start + ( self.target - self.segment_start ) * eased;
		if self.segment_elapsed >= self.segment_duration { self.progress = self.target; }
	}
}


impl AnimationFrame {
	pub fn overlay_opacity( &self, maximum_percent: u8 ) -> u8 {
		( smooth_step( self.progress ) * maximum_percent.min( 100 ) as f32 * 2.55 ).round() as u8
	}


	pub fn transition_opacity( &self ) -> u8 {
		( ( 1.0 - smooth_step( ( self.progress / 0.68 ).clamp( 0.0, 1.0 ) ) ) * 255.0 ).round() as u8
	}


	pub fn group_progress( &self, group_index: usize ) -> f32 {
		smooth_step( staggered_progress( self.progress, group_index.min( 4 ), 0.012 ) )
	}


	pub fn tile_progress( &self, tile_index: usize ) -> f32 {
		smooth_step( staggered_progress( self.progress, tile_index.min( MAX_STAGGER_INDEX ), 0.024 ) )
	}


	pub fn tile_opacity( &self, tile_index: usize ) -> f32 {
		smooth_step( staggered_progress( self.progress, tile_index.min( MAX_STAGGER_INDEX ), 0.016 ) )
	}
}


fn staggered_progress( progress: f32, index: usize, initial_delay: f32 ) -> f32 {
	let delay = initial_delay + index as f32 * TILE_DELAY_NORMALIZED;
	( ( progress - delay ) / ( 1.0 - delay ) ).clamp( 0.0, 1.0 )
}


fn ease_out_cubic( value: f32 ) -> f32 {
	1.0 - ( 1.0 - value ).powi( 3 )
}


fn smooth_step( value: f32 ) -> f32 {
	value * value * ( 3.0 - 2.0 * value )
}


#[cfg( test )]
mod tests {
	use super::*;


	#[test]
	fn reversing_starts_from_the_current_visual_position() {
		let mut animation = AnimationController::new( 1000 );
		animation.open();
		animation.advance_by( 0.35 );
		let progress = animation.progress;
		animation.close();
		assert_eq!( animation.state(), VisibilityState::Closing );
		assert_eq!( animation.progress, progress );
		animation.advance_by( 0.05 );
		assert!( animation.progress < progress );
	}


	#[test]
	fn reverse_duration_is_proportional_to_remaining_distance() {
		let mut animation = AnimationController::new( 1000 );
		animation.open();
		animation.advance_by( 0.2 );
		let remaining = animation.progress;
		animation.close();
		assert!( ( animation.segment_duration - remaining ).abs() < 0.0001 );
		animation.advance_by( remaining );
		assert_eq!( animation.state(), VisibilityState::Hidden );
	}
}
