// Lokalni testni okvir: prevodi i pokreće wips/lagani/* sa zamjenama za RustDesk jezgru.
//   lagani-proba            početni prozor (ID 121179531, spojeno)
//   lagani-proba --cm       prozor za odobravanje (nakon 1,5 s stiže lažni zahtjev)
#![allow(dead_code, non_snake_case)]

pub const VERSION: &str = "1.5.1";

pub mod ipc {
    pub fn get_id() -> String {
        "121179531".to_owned()
    }
}

pub mod platform {
    pub fn quit_gui() {
        std::process::exit(0);
    }
}

pub mod ui_interface {
    pub struct UiStatus {
        pub status_num: i32,
        pub id: String,
    }
    pub fn get_connect_status() -> UiStatus {
        UiStatus { status_num: 1, id: "121179531".to_owned() }
    }
    pub fn start_option_status_sync() {}
}

pub mod ui_cm_interface {
    pub struct Client {
        pub id: i32,
        pub name: String,
        pub peer_id: String,
        pub authorized: bool,
    }
    #[derive(Clone)]
    pub struct ConnectionManager<T: InvokeUiCM> {
        pub ui_handler: T,
    }
    pub trait InvokeUiCM: Send + Clone + 'static + Sized {
        fn add_connection(&self, client: &Client);
        fn remove_connection(&self, id: i32, close: bool);
        fn new_message(&self, id: i32, text: String);
        fn change_theme(&self, dark: String);
        fn change_language(&self);
        fn show_elevation(&self, show: bool);
        fn update_voice_call_state(&self, client: &Client);
        fn file_transfer_log(&self, action: &str, log: &str);
    }
    pub fn start_ipc<T: InvokeUiCM>(cm: ConnectionManager<T>) {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        cm.ui_handler.add_connection(&Client {
            id: 1,
            name: "Tehničar".to_owned(),
            peer_id: "987654321".to_owned(),
            authorized: false,
        });
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }
    pub fn authorize(_id: i32) {}
    pub fn close(_id: i32) {
        std::process::exit(0);
    }
    pub fn remove(_id: i32) {}
    pub fn get_clients_length() -> usize {
        1
    }
}

pub mod lagani {
    #[path = "../../../cm.rs"]
    mod cm;
    #[path = "../../../d2d.rs"]
    mod d2d;
    #[path = "../../../glavni.rs"]
    mod glavni;
    #[path = "../../../izgled.rs"]
    mod izgled;
    #[path = "../../../win32.rs"]
    mod win32;

    use hbb_common::log;

    pub fn start(args: &mut [String]) {
        log::info!("lagani: start, args {:?}", args);
        if args.is_empty() {
            glavni::run();
        } else if args[0] == "--cm" {
            cm::run();
        } else {
            glavni::run();
        }
    }
}

fn main() {
    unsafe {
        let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
    }
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    lagani::start(&mut args);
}
