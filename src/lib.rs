pub mod device_manager;
pub mod client;

pub use device_manager::DeviceManager;
pub use client::UsbIO;

use wire_weaver::wire_weaver_api;

#[wire_weaver_api(
    ww = "../usb_io_ww/usb_io_v0_1.ww",
    api_model = "client_server_v0_1",
    // skip_model_codegen = true
    client = true,
    no_alloc = false,
    debug_to_file = "target/out.rs",
)]
mod raw_client {
    pub struct Client {

    }
}