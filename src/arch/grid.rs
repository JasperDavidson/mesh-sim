use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::arch::node::MeshNode;
use crate::comm::packet::{Event, Packet};
use crate::comm::transfer::NodeCommError;
use crate::comm::transfer::{calc_path, receive_packets, send_packet};

const LINK_BUFFER_SIZE: usize = 2;
const INNER_BUFFER_SIZE: usize = 4;

#[derive(Error, Debug)]
pub enum GridAccessError {
    #[error("Invalid width, error accessing column: {0}")]
    InvalidWidth(u8),

    #[error("Invalid height, error accessing row: {0}")]
    InvalidHeight(u8),
}

// Should figure out how to align each MeshNode at 64 byte boundary to avoid false sharing
#[derive(Default)]
pub struct Grid {
    // nodes: Vec<Vec<Arc<MeshNode>>>,
    nodes: Arc<[Arc<[MeshNode]>]>,
}

impl Grid {
    // Need a robust way to build a grid of nodes of dimensions width x height
    // - How will we handle instantiating the channels?
    // - Maybe doing a transmit pass where we build all the channels and store the transmitters in
    // the structs and store the receivers in a map based on coordinate, then a receive pass that
    // checks the map for each node and provides the receivers
    pub fn init_grid(&mut self, width: u8, height: u8) -> UnboundedReceiver<Event> {
        // Collection to hold rx channels for second connection pass
        #[derive(Default)]
        struct ChannelHolder {
            rx_up: Option<Receiver<Packet>>,
            rx_down: Option<Receiver<Packet>>,
            rx_left: Option<Receiver<Packet>>,
            rx_right: Option<Receiver<Packet>>,
            tx_up: Option<Sender<Packet>>,
            tx_down: Option<Sender<Packet>>,
            tx_left: Option<Sender<Packet>>,
            tx_right: Option<Sender<Packet>>,
        }
        let mut rx_local_map: HashMap<(u8, u8), Receiver<Packet>> = HashMap::new();
        let mut channel_map: HashMap<(u8, u8), ChannelHolder> = HashMap::new();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();

        let mut temp_nodes: Vec<Vec<MeshNode>> = Vec::new();
        for y in 0..height {
            let mut grid_row = Vec::new();

            for x in 0..width {
                let (tx_local, rx_local) = mpsc::channel::<Packet>(INNER_BUFFER_SIZE);
                let cur_node = MeshNode::init_channeless(x, y, 10, 10, tx_local);
                rx_local_map.insert((x, y), rx_local);

                // Create channels based on position
                if y < height {
                    let (tx_down, rx_down) = mpsc::channel::<Packet>(LINK_BUFFER_SIZE);

                    let cur_node = channel_map.entry((x, y)).or_default();
                    cur_node.tx_down = Some(tx_down);
                    let up_node = channel_map.entry((x, y + 1)).or_default();
                    up_node.rx_up = Some(rx_down);
                }
                if y > 0 {
                    let (tx_up, rx_up) = mpsc::channel::<Packet>(LINK_BUFFER_SIZE);

                    let cur_node = channel_map.entry((x, y)).or_default();
                    cur_node.tx_up = Some(tx_up);
                    let down_node = channel_map.entry((x, y - 1)).or_default();
                    down_node.rx_down = Some(rx_up);
                }
                if x > 0 {
                    let (tx_left, rx_left) = mpsc::channel::<Packet>(LINK_BUFFER_SIZE);

                    let cur_node = channel_map.entry((x, y)).or_default();
                    cur_node.tx_left = Some(tx_left);
                    let right_node = channel_map.entry((x - 1, y)).or_default();
                    right_node.rx_right = Some(rx_left);
                }
                if x < width {
                    let (tx_right, rx_right) = mpsc::channel::<Packet>(LINK_BUFFER_SIZE);

                    let cur_node = channel_map.entry((x, y)).or_default();
                    cur_node.tx_right = Some(tx_right);
                    let left_node = channel_map.entry((x + 1, y)).or_default();
                    left_node.rx_left = Some(rx_right);
                }

                grid_row.push(cur_node);
            }

            temp_nodes.push(grid_row);
        }

        // Create the Arc MeshNode holder for thread safe access + optimized space usage
        let inner_nodes_arc: Vec<Arc<[MeshNode]>> = temp_nodes
            .into_iter()
            .map(|node_row| Arc::from(node_row.into_boxed_slice()))
            .collect();
        self.nodes = Arc::from(inner_nodes_arc.into_boxed_slice());

        for y in 0..height {
            for x in 0..width {
                let mut tx_up = None;
                let mut tx_down = None;
                let mut tx_left = None;
                let mut tx_right = None;
                let mut rx_up = None;
                let mut rx_down = None;
                let mut rx_left = None;
                let mut rx_right = None;

                if let Some(rx_conns) = channel_map.get_mut(&(x, y)) {
                    tx_up = rx_conns.tx_up.clone();
                    tx_down = rx_conns.tx_down.clone();
                    tx_left = rx_conns.tx_left.clone();
                    tx_right = rx_conns.tx_right.clone();
                    rx_up = std::mem::take(&mut rx_conns.rx_up);
                    rx_down = std::mem::take(&mut rx_conns.rx_down);
                    rx_left = std::mem::take(&mut rx_conns.rx_left);
                    rx_right = std::mem::take(&mut rx_conns.rx_right);
                }

                let (inner_tx_up, inner_rx_up) = mpsc::channel(INNER_BUFFER_SIZE);
                let (inner_tx_down, inner_rx_down) = mpsc::channel(INNER_BUFFER_SIZE);
                let (inner_tx_left, inner_rx_left) = mpsc::channel(INNER_BUFFER_SIZE);
                let (inner_tx_right, inner_rx_right) = mpsc::channel(INNER_BUFFER_SIZE);

                if let Some(inner_rx_local) = rx_local_map.remove(&(x, y)) {
                    let tx_event_receive = event_tx.clone();
                    let tx_event_send = event_tx.clone();
                    tokio::spawn(async move {
                        receive_packets(
                            rx_up,
                            rx_down,
                            rx_left,
                            rx_right,
                            &inner_tx_up,
                            &inner_tx_down,
                            &inner_tx_left,
                            &inner_tx_right,
                            &tx_event_receive,
                        )
                        .await
                    });

                    tokio::spawn(async move {
                        send_packet(
                            tx_up.as_ref(),
                            tx_down.as_ref(),
                            tx_left.as_ref(),
                            tx_right.as_ref(),
                            &tx_event_send,
                            inner_rx_up,
                            inner_rx_down,
                            inner_rx_left,
                            inner_rx_right,
                            inner_rx_local,
                        )
                        .await
                    });
                } else {
                    panic!("Failed to retrieve inner rx for node ({x}, {y})");
                }
            }
        }

        event_rx
    }

    pub fn access_node(&self, (x, y): (u8, u8)) -> Result<&MeshNode, GridAccessError> {
        let node_row = self
            .nodes
            .get(y as usize)
            .ok_or_else(|| GridAccessError::InvalidHeight(x))?;
        let node = node_row
            .get(x as usize)
            .ok_or_else(|| GridAccessError::InvalidWidth(y))?;

        Ok(&node)
    }

    // Takes a packet and sends it from a src node to a destination node
    // Calculates the first direction and enquues in that mpsc tx, nodes carry from there
    pub async fn send_packet_grid(
        node: &MeshNode,
        mut packet: Packet,
        dest_pos: (u8, u8),
    ) -> Result<(), NodeCommError> {
        packet.header.destination = dest_pos;
        packet.header.path = calc_path((node.x, node.y), dest_pos);

        node.tx_local.send(packet).await?;

        Ok(())
    }
}
