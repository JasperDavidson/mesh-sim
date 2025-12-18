use tokio::sync::mpsc::Sender;

use crate::comm::packet::Packet;

// Need some way to handle data transfer
pub struct MeshNode {
    pub x: u8,
    pub y: u8,
    pub tx_rate: u64,
    pub rx_rate: u64,
    pub tx_local: Sender<Packet>,
}

impl MeshNode {
    pub fn init_channeless(
        x: u8,
        y: u8,
        tx_rate: u64,
        rx_rate: u64,
        tx_local: Sender<Packet>,
    ) -> Self {
        Self {
            x,
            y,
            tx_rate,
            rx_rate,
            tx_local,
        }
    }
}
