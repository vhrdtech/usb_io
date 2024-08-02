use tracing::info;
use wire_weaver::shrink_wrap::BufWriter;
// use wire_weaver::wire_weaver;
use wire_weaver_usb::{AsyncDeviceHandle, Command, DeviceManagerEvent, Event, UsbDeviceManager};
use crate::UsbIO;
// wire_weaver!("ww/test.ww");

pub struct DeviceManager {
    usb_dm: UsbDeviceManager,
    connections: Vec<Connection>,
}

impl DeviceManager {
    pub fn new() -> Self {
        DeviceManager {
            usb_dm: UsbDeviceManager::new(),
            connections: vec![],
        }
    }

    pub fn handle_events(&mut self) {
        if let Some(ev) = self.usb_dm.poll() {
            match ev {
                DeviceManagerEvent::Opened(handle) => {
                    info!("Got usb async handle");
                    self.connections.push(Connection::Usb { handle })
                }
                DeviceManagerEvent::DeviceConnected => {}
                DeviceManagerEvent::DeviceDisconnected => {}
            }
        }

        let mut modify = vec![];
        for (idx, c) in self.connections.iter_mut().enumerate() {
            match c {
                Connection::Usb { handle } => {
                    while let Some(ev) = handle.try_recv() {
                        match ev {
                            Event::Received(buf) => {
                                info!("rx buf: {buf:?}");
                            }
                            Event::RecycleBuffer(_) => {}
                            Event::Disconnected => {
                                info!("disconnected");
                                modify.push((idx, Connection::UsbDisconnected { usb_dev: () }));
                                break;
                            }
                        }
                    }
                }
                Connection::UsbDisconnected { .. } => {}
            }
        }
        for (idx, c) in modify {
            self.connections[idx] = c;
        }
    }

    // TODO: make async and add timeout and error
    pub fn connect_auto(&mut self) {
        self.usb_dm.connect();
    }

    pub fn disconnect(&mut self) {
        for c in &mut self.connections {
            if let Connection::Usb { handle } = c {
                handle.commands_tx.try_send(Command::Close).unwrap();
            }
        }
    }

    pub fn connect_result(&mut self) -> Result<UsbIO, &'static str> {
        if let Some(connection) = self.connections.pop() {
            Ok(UsbIO::new(connection))
        } else {
            Err("No devices connected")
        }
    }
}

pub(crate) enum Connection {
    Usb { handle: AsyncDeviceHandle },
    UsbDisconnected { usb_dev: () },
    // Ethernet,
}
