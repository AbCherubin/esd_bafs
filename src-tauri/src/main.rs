#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use regex::Regex;
use serialport::{available_ports, ClearBuffer, SerialPortType};
use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tauri::{CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};
use tauri::{Manager, Window};
use tauri_plugin_autostart::MacosLauncher;
// Define a payload struct
#[derive(serde::Serialize, Clone)]
struct SerialDataPayload {
    data: String,
}
#[derive(serde::Serialize)]
struct SerialPortInfoWrapper {
    port_name: String,
    port_type: SerialPortTypeWrapper,
}

#[derive(serde::Serialize)]
enum SerialPortTypeWrapper {
    UsbType,
    BluetoothType,
    PciType,
    UnknownType,
}
#[derive(Clone, serde::Serialize)]
struct Payload {
    args: Vec<String>,
    cwd: String,
}
#[derive(Debug)]
struct Signal {
    reason: String,
}

#[tauri::command]
async fn start_serial_communication(port_name: String, baud_rate: u32, window: Window) -> () {
    let port = serialport::new(&port_name, baud_rate)
        .timeout(Duration::from_millis(10))
        .open();
    let name_regex = Regex::new(r#"\*#ESD_(?P<name>[^#]+)#"#).unwrap();
    let target_msg = format!("{}{}", "*#ESD_OFF#", "\r\n");
    match port {
        Ok(mut port) => {
            let mut serial_buf: Vec<u8> = vec![0; 1000];
            println!("Receiving data on {} at {} baud:", &port_name, &baud_rate);

            let (tx, rx) = mpsc::channel::<Signal>();
            let tx1 = tx.clone();
            let tx2 = tx.clone();

            let id_stop = window.listen("stopSerial", move |_| {
                println!("Received stop signal ");
                match tx1.send(Signal {
                    reason: "stopSerial".to_string(),
                }) {
                    Ok(()) => {
                        println!("Sent stop signal to serial communication thread.");
                    }
                    Err(e) => {
                        // Failed to send message

                        eprintln!("Error sending stop signal: {:?}", e);
                        // Handle the error accordingly
                    }
                }
            });

            let id_send_command = window.listen("sendCommand", move |_| {
                println!("Send Command signal ");
                match tx2.send(Signal {
                    reason: "sendCommand".to_string(),
                }) {
                    Ok(()) => {
                        println!("Sent Cmmand signal to serial communication thread.");
                    }
                    Err(e) => {
                        // Failed to send message
                        eprintln!("Error sending stop signal: {:?}", e);
                        // Handle the error accordingly
                    }
                }
            });

            window.emit_all("portState", "Connected").unwrap();

            thread::spawn(move || {
                loop {
                    match rx.try_recv() {
                        Ok(signal) => {
                            // Stop signal received, extract the reason
                            let reason = signal.reason;
                            if reason == "stopSerial" {
                                port.clear(ClearBuffer::Input)
                                    .expect("Failed to discard input buffer");

                                break;
                            } else if reason == "sendCommand" {
                                match port.write(target_msg.as_bytes()) {
                                    Ok(_) => {
                                        window
                                            .emit_all(
                                                "alertData",
                                                SerialDataPayload {
                                                    data: "".to_string(),
                                                },
                                            )
                                            .unwrap();
                                    }
                                    Err(e) => {
                                        eprintln!("Write failed: {:?}", e);
                                        // Handle the write failure accordingly
                                    }
                                }
                            }
                            println!("Received stop signal with reason: {}", reason);
                            // Handle the stop signal accordingly
                        }

                        Err(_e) => {}
                    }
                    match port.read(serial_buf.as_mut_slice()) {
                        Ok(t) => {
                            let data = String::from_utf8_lossy(&serial_buf[..t]).to_string();
                            // Emit a global event with the received data
                            if let Some(captures) = name_regex.captures(&data) {
                                if let Some(name) = captures.name("name") {
                                    println!("Received valid data with name: {}", name.as_str());
                                    window.unminimize().unwrap();
                                    window.show().unwrap();
                                    window.set_focus().unwrap();
                                    window
                                        .emit_all(
                                            "alertData",
                                            SerialDataPayload {
                                                data: name.as_str().to_string(),
                                            },
                                        )
                                        .unwrap();
                                }
                            } else {
                                println!("Received data does not match the expected format");
                            }
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                        Err(e) => {
                            eprintln!("{:?}", e);
                            break;
                        }
                    }
                }
                window.unlisten(id_send_command);
                window.unlisten(id_stop);
                window.emit_all("portState", "Disconnected").unwrap();
                println!("Exit {}", &port_name);
            });
        }
        Err(e) => {
            eprintln!("Failed to open \"{}\". Error: {}", port_name, e);
            window.emit_all("portState", "Port Busy").unwrap();
        }
    }
}

// Tauri command to list available serial ports
#[tauri::command]
fn list_serial_ports() -> Vec<SerialPortInfoWrapper> {
    available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|port| SerialPortInfoWrapper {
            port_name: port.port_name,
            port_type: match port.port_type {
                SerialPortType::UsbPort(_usbinfo) => SerialPortTypeWrapper::UsbType,
                SerialPortType::BluetoothPort => SerialPortTypeWrapper::BluetoothType,
                SerialPortType::PciPort => SerialPortTypeWrapper::PciType,
                SerialPortType::Unknown => SerialPortTypeWrapper::UnknownType,
            },
        })
        .collect()
}

#[tokio::main]
async fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide");
    let tray_menu = SystemTrayMenu::new()
        .add_item(quit)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(hide);
    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    std::process::exit(0);
                }
                "hide" => {
                    let window = app.get_window("main").unwrap();

                    let item_handle = app.tray_handle().get_item(&id);
                    if window.is_visible().unwrap() {
                        window.hide().unwrap();
                        item_handle.set_title("Show").unwrap();
                    } else {
                        window.show().unwrap();
                        item_handle.set_title("Hide").unwrap();
                    }
                }

                _ => {}
            },

            _ => {}
        })
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                event
                    .window()
                    .app_handle()
                    .tray_handle()
                    .get_item("hide")
                    .set_title("Show")
                    .unwrap();
                event.window().hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            list_serial_ports,
            start_serial_communication,
        ])
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!("{}, {argv:?}, {cwd}", app.package_info().name);

            app.emit_all("single-instance", Payload { args: argv, cwd })
                .unwrap();
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--flag1", "--flag2"]),
        ))
        .run(tauri::generate_context!())
        .expect("Error while running Tauri application");
}
