use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, oneshot};
use wire_weaver::{ProtocolInfo, wire_weaver_api};
use wire_weaver_client_server::{Command, Error};
use wire_weaver_usb_host::wire_weaver_client_server::OnError;
use wire_weaver_usb_host::{
    ConnectionInfo, UsbDeviceFilter, UsbError, usb_worker, wire_weaver_client_server,
};

#[derive(Clone)]
pub struct B193Driver<F, E> {
    args_scratch: [u8; 512],
    cmd_tx: mpsc::UnboundedSender<Command<F, E>>,
    pub conn_state: Arc<RwLock<ConnectionInfo>>,
}

pub type USBIODriver = B193Driver<UsbDeviceFilter, UsbError>;

impl USBIODriver {
    pub async fn connect(
        filter: UsbDeviceFilter,
        on_error: OnError,
        max_hs_usb_packet_size: usize,
    ) -> Result<Self, Error<UsbError>> {
        let conn_state = Arc::new(RwLock::new(ConnectionInfo::default()));
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let user_protocol = ProtocolInfo {
            protocol_id: no_alloc_client::PROTOCOL_GID,
            major_version: no_alloc_client::VERSION_MAJOR,
            minor_version: no_alloc_client::VERSION_MINOR,
        };
        let conn_state_clone = conn_state.clone();
        tokio::spawn(async move {
            usb_worker(
                cmd_rx,
                conn_state_clone,
                user_protocol,
                max_hs_usb_packet_size,
            )
            .await;
        });
        let (connected_tx, connected_rx) = oneshot::channel();
        cmd_tx
            .send(Command::Connect {
                filter,
                on_error: on_error.into(),
                connected_tx: Some(connected_tx),
            })
            .map_err(|_| Error::EventLoopNotRunning)?;
        let connection_result = connected_rx.await.map_err(|_| Error::EventLoopNotRunning)?;
        connection_result?;
        Ok(Self {
            args_scratch: [0; 512],
            cmd_tx,
            conn_state,
        })
    }
}

#[wire_weaver_api(
    ww = "../vb193_usb_io_fw/ww/b193_usb_io.ww",
    // api_model = "client_server_v0_1",
    client = true,
    no_alloc = false,
    use_async = true,
    derive = "Debug",
    debug_to_file = "./target/ww_no_alloc.rs"
)]
pub(crate) mod no_alloc_client {
    use super::B193Driver as Client;
    use super::wire_weaver_client_server;
}
