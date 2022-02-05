#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]
#![feature(div_duration)]
#![feature(more_qualified_paths)]

mod slider_io;

use std::sync::{Arc, Mutex};

use env_logger;
use log::info;

use tauri::{
  async_runtime::Handle as AsyncHandle, AppHandle, CustomMenuItem, Event, Manager, Runtime,
  SystemTray, SystemTrayEvent, SystemTrayMenu,
};

fn show_window<R: Runtime>(handle: &AppHandle<R>) {
  handle.get_window("main").unwrap().show().ok();
}

fn hide_window<R: Runtime>(handle: &AppHandle<R>) {
  handle.get_window("main").unwrap().hide().ok();
}

fn quit_app() {
  std::process::exit(0);
}

fn main() {
  // Setup logger
  env_logger::Builder::new()
    .filter_level(log::LevelFilter::Debug)
    .init();

  let config = Arc::new(Mutex::new(Some(slider_io::Config::default())));
  let manager: Arc<Mutex<Option<slider_io::Manager>>> = Arc::new(Mutex::new(None));
  {
    let config_handle = config.lock().unwrap();
    let config_handle_ref = config_handle.as_ref().unwrap();
    config_handle_ref.save();
    // let mut manager_handle = manager.lock().unwrap();
    // manager_handle.take();
    // manager_handle.replace(slider_io::Manager::new(config_handle_ref.clone()));
  }

  tauri::Builder::default()
    .system_tray(
      // System tray content
      SystemTray::new().with_menu(
        SystemTrayMenu::new()
          .add_item(CustomMenuItem::new("slidershim".to_string(), "slidershim").disabled())
          .add_item(CustomMenuItem::new("show".to_string(), "Show"))
          .add_item(CustomMenuItem::new("quit".to_string(), "Quit")),
      ),
    )
    .on_system_tray_event(|app_handle, event| match event {
      // System tray events
      SystemTrayEvent::LeftClick {
        position: _,
        size: _,
        ..
      } => {
        show_window(app_handle);
      }
      SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
        "show" => {
          show_window(app_handle);
        }
        "quit" => {
          quit_app();
        }
        _ => {
          panic!("Unexpected menu item click {}", id.as_str());
        }
      },
      _ => {}
    })
    .setup(move |app| {
      // Before app starts

      // Hide event
      let app_handle = app.handle();
      app.listen_global("hide", move |_| {
        hide_window(&app_handle);
      });

      // Quit event
      app.listen_global("quit", |_| {
        quit_app();
      });

      // UI ready event
      let app_handle = app.handle();
      let config_clone = Arc::clone(&config);
      app.listen_global("heartbeat", move |_| {
        let handle = AsyncHandle::try_current();
        println!("handle, {:?}", handle);

        let config_handle = config_clone.lock().unwrap();
        info!("Heartbeat received");
        app_handle
          .emit_all(
            "showConfig",
            Some(config_handle.as_ref().unwrap().raw.as_str().to_string()),
          )
          .unwrap();
      });

      // Config set event
      let config_clone = Arc::clone(&config);
      let manager_clone = Arc::clone(&manager);
      app.listen_global("setConfig", move |event| {
        let payload = event.payload().unwrap();
        info!("Config applied {}", payload);
        if let Some(new_config) = slider_io::Config::from_str(payload) {
          let mut config_handle = config_clone.lock().unwrap();
          config_handle.take();
          config_handle.replace(new_config);
          let config_handle_ref = config_handle.as_ref().unwrap();
          config_handle_ref.save();
          let mut manager_handle = manager_clone.lock().unwrap();
          manager_handle.take();
          manager_handle.replace(slider_io::Manager::new(config_handle_ref.clone()));
        }
      });

      Ok(())
    })
    .build(tauri::generate_context!())
    .expect("error while running tauri application")
    .run(|app_handle, event| match event {
      // After app starts
      Event::CloseRequested { label, api, .. } if label.as_str() == "main" => {
        api.prevent_close();
        hide_window(app_handle);
      }
      _ => {}
    });
}
