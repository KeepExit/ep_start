//! ::  Project Path  ->  ep_start :: lib.rs :: main
//! ::  Created User  ->  Studio :: Ep
//! ::  Created Time  ->  2026/6/19 22:30 周五

pub mod injector_bridge;


use configuration::ConfigurationStore;
use localization::TextResources;
use platform::{ PlatformRuntime, TrayEvent, TrayIcon, TrayIconConfig, TrayMenuEntry };
use settings::SettingsRuntime;
use start::StartRuntime;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;


const TRAY_COMMAND_SETTINGS: u16 = 1;
const TRAY_COMMAND_EXIT: u16 = 2;
const TRAY_ICON_SMALL: &[ u8 ] = include_bytes!( "../../../assets/Omelette.ico" );
const TRAY_ICON_LARGE: &[ u8 ] = include_bytes!( "../../../assets/Omelette x256.ico" );


pub struct Application {
	_shell_injection: crate_injector::ShellInjection,
	_tray: TrayIcon,
	_settings: SettingsRuntime,
	_start: StartRuntime,
	platform: PlatformRuntime,
}


impl Application {
	pub fn new() -> Result< Self, String > {
		let platform = PlatformRuntime::new()?;
		let text = TextResources::system()?;
		let configuration = ConfigurationStore::discover()?;
		let preferences = configuration.load()?;
		let start = StartRuntime::new( &platform, preferences.start )?;
		let shell_injection = injector_bridge::install()?;
		let start_controller = start.controller();
		let settings = SettingsRuntime::new( configuration, preferences, TRAY_ICON_SMALL, TRAY_ICON_LARGE, move |preferences| start_controller.update_preferences( preferences ) )?;
		let settings_controller = settings.controller();
		let tray_config = TrayIconConfig { tooltip: "ep_start".to_string(), small_icon: TRAY_ICON_SMALL, large_icon: TRAY_ICON_LARGE, menu: vec![ TrayMenuEntry::command( TRAY_COMMAND_SETTINGS, text.open_settings ), TrayMenuEntry::separator(), TrayMenuEntry::command( TRAY_COMMAND_EXIT, text.exit ) ] };
		let tray = TrayIcon::create( tray_config, move |event| match event {
			TrayEvent::Activate => start_controller.toggle_from_tray(),
			TrayEvent::Command( TRAY_COMMAND_SETTINGS ) => settings_controller.show(),
			TrayEvent::Command( TRAY_COMMAND_EXIT ) => unsafe { PostQuitMessage( 0 ) },
			TrayEvent::Command( _ ) => {}
		} )?;
		Ok( Self { _shell_injection: shell_injection, _tray: tray, _settings: settings, _start: start, platform } )
	}


	pub fn run( &self ) -> Result< (), String > {
		self.platform.run()
	}
}


pub fn run() -> Result< (), String > {
	if platform::run_shell_restart_helper_if_requested()? { return Ok( () ); }
	if !platform::ensure_elevated()? { return Ok( () ); }
	Application::new()?.run()
}


pub fn show_fatal_error( message: &str ) {
	platform::show_error_dialog( "ep_start 启动失败", message );
}
