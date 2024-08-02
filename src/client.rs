use wire_weaver_usb::Command;
use crate::device_manager::Connection;
use crate::raw_client::Client as RawClient;

pub struct UsbIO {
    connection: Connection,
    raw: RawClient,
}

impl UsbIO {
    pub(crate) fn new(connection: Connection) -> Self {
        UsbIO {
            connection,
            raw: RawClient::new()
        }
    }

    pub fn led_on(&mut self) {
        let request_bytes = self.raw.led_on();
        if let Connection::Usb { handle } = &mut self.connection {
            handle
                .commands_tx
                .try_send(Command::Send(request_bytes))
                .unwrap();
        }
    }

    pub fn led_off(&mut self) {
        let request_bytes = self.raw.led_off();
        if let Connection::Usb { handle } = &mut self.connection {
            handle
                .commands_tx
                .try_send(Command::Send(request_bytes))
                .unwrap();
        }
    }
}