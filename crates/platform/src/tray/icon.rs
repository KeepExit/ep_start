//! ::  Project Path  ->  ep_start :: icon.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/20 15:35 周六


use windows::Win32::UI::WindowsAndMessaging::{ CreateIconFromResourceEx, DestroyIcon, GetSystemMetrics, HICON, LR_DEFAULTCOLOR, SM_CXSMICON, SM_CYSMICON };


pub struct EmbeddedIcon {
	handle: HICON,
}


impl EmbeddedIcon {
	pub fn load( small_icon: &[ u8 ], large_icon: &[ u8 ] ) -> Result< Self, String > {
		let width = unsafe { GetSystemMetrics( SM_CXSMICON ) }.max( 16 );
		let height = unsafe { GetSystemMetrics( SM_CYSMICON ) }.max( 16 );
		let source = if width <= 16 && height <= 16 { small_icon } else { large_icon };
		let image = first_image( source )?;
		let handle = unsafe { CreateIconFromResourceEx( image, true, 0x00030000, width, height, LR_DEFAULTCOLOR ) }.map_err( |error| format!( "加载托盘图标失败：{}", error ) )?;
		Ok( Self { handle } )
	}


	pub fn load_for_size( source: &[ u8 ], width: i32, height: i32 ) -> Result< Self, String > {
		let image = first_image( source )?;
		let handle = unsafe { CreateIconFromResourceEx( image, true, 0x00030000, width, height, LR_DEFAULTCOLOR ) }.map_err( |error| format!( "加载窗口图标失败：{}", error ) )?;
		Ok( Self { handle } )
	}


	pub fn handle( &self ) -> HICON {
		self.handle
	}
}


impl Drop for EmbeddedIcon {
	fn drop( &mut self ) {
		if !self.handle.is_invalid() { unsafe { let _ = DestroyIcon( self.handle ); } }
	}
}


fn first_image( data: &[ u8 ] ) -> Result< &[ u8 ], String > {
	if data.len() < 22 || read_u16( data, 0 ) != Some( 0 ) || read_u16( data, 2 ) != Some( 1 ) || read_u16( data, 4 ).unwrap_or( 0 ) == 0 { return Err( "托盘图标不是有效的 ICO 文件".to_string() ); }
	let size = read_u32( data, 14 ).ok_or_else( || "托盘图标目录损坏".to_string() )? as usize;
	let offset = read_u32( data, 18 ).ok_or_else( || "托盘图标目录损坏".to_string() )? as usize;
	let end = offset.checked_add( size ).ok_or_else( || "托盘图标尺寸溢出".to_string() )?;
	data.get( offset..end ).ok_or_else( || "托盘图标数据不完整".to_string() )
}


fn read_u16( data: &[ u8 ], offset: usize ) -> Option< u16 > {
	Some( u16::from_le_bytes( data.get( offset..offset + 2 )?.try_into().ok()? ) )
}


fn read_u32( data: &[ u8 ], offset: usize ) -> Option< u32 > {
	Some( u32::from_le_bytes( data.get( offset..offset + 4 )?.try_into().ok()? ) )
}


#[cfg( test )]
mod tests {
	use super::first_image;


	#[test]
	fn rejects_truncated_icon() {
		assert!( first_image( &[ 0, 0, 1, 0, 1, 0 ] ).is_err() );
	}
}
