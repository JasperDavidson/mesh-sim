use crate::comm::packet::Packet;
use tokio::sync::mpsc::{Receiver, Sender};

// Need some way to handle data transfer
pub struct MeshNode {
    pub tx_rate: u64,
    pub rx_rate: u64,

    // Sender and Receiver handles from/to each direction
    // Formed as Options since boundary nodes may lack them
    pub(crate) tx_up: Option<Sender<Packet>>,
    pub(crate) tx_down: Option<Sender<Packet>>,
    pub(crate) tx_left: Option<Sender<Packet>>,
    pub(crate) tx_right: Option<Sender<Packet>>,

    pub(crate) rx_up: Option<Receiver<Packet>>,
    pub(crate) rx_down: Option<Receiver<Packet>>,
    pub(crate) rx_left: Option<Receiver<Packet>>,
    pub(crate) rx_right: Option<Receiver<Packet>>,
}

impl MeshNode {
    pub fn init_channeless(tx_rate: u64, rx_rate: u64) -> Self {
        Self {
            tx_rate,
            rx_rate,
            tx_up: None,
            tx_down: None,
            tx_left: None,
            tx_right: None,
            rx_up: None,
            rx_down: None,
            rx_left: None,
            rx_right: None,
        }
    }
}
