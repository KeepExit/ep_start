//! ::  Project Path  ->  ep_start :: build.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/21 18:55 周日


use std::path::PathBuf;


fn main() {
	let manifest = PathBuf::from( env!( "CARGO_MANIFEST_DIR" ) ).join( "../../assets/app.manifest" );
	println!( "cargo:rerun-if-changed={}", manifest.display() );
	println!( "cargo:rustc-link-arg-bin=app=/MANIFEST:EMBED" );
	println!( "cargo:rustc-link-arg-bin=app=/MANIFESTUAC:level='asInvoker' uiAccess='false'" );
	println!( "cargo:rustc-link-arg-bin=app=/MANIFESTINPUT:{}", manifest.display() );
}
