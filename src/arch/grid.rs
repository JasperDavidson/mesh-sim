use std::collections::HashMap;
use std::mem;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

use crate::arch::node::MeshNode;
use crate::comm::packet::Packet;
use crate::comm::transfer::NodeCommError;
use crate::comm::transfer::receive_packets;
use crate::comm::transfer::send_packet;

const BUFFER_SIZE: usize = 32;

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
    nodes: Arc<[Arc<[Arc<MeshNode>]>]>,
}

impl Grid {
    // Need a robust way to build a grid of nodes of dimensions width x height
    // - How will we handle instantiating the channels?
    // - Maybe doing a transmit pass where we build all the channels and store the transmitters in
    // the structs and store the receivers in a map based on coordinate, then a receive pass that
    // checks the map for each node and provides the receivers
    pub fn init_grid(&mut self, width: u8, height: u8) {
        // Collection to hold rx channels for second connection pass
        #[derive(Default)]
        struct RxHolder {
            up: Option<Receiver<Packet>>,
            down: Option<Receiver<Packet>>,
            left: Option<Receiver<Packet>>,
            right: Option<Receiver<Packet>>,
        }
        let mut rx_map: HashMap<(u8, u8), RxHolder> = HashMap::new();

        let mut temp_nodes: Vec<Vec<Arc<MeshNode>>> = Vec::new();
        for y in 0..height {
            let mut grid_row = Vec::new();

            for x in 0..width {
                let mut cur_node = MeshNode::init_channeless(x, y, 10, 10);

                // Create channels based on position
                if y < height {
                    let (tx_down, rx_down) = mpsc::channel::<Packet>(BUFFER_SIZE);
                    cur_node.tx_down = Some(tx_down);

                    let up_node = rx_map.entry((x, y + 1)).or_default();
                    up_node.up = Some(rx_down);
                }
                if y > 0 {
                    let (tx_up, rx_up) = mpsc::channel::<Packet>(BUFFER_SIZE);
                    cur_node.tx_up = Some(tx_up);

                    let down_node = rx_map.entry((x, y - 1)).or_default();
                    down_node.down = Some(rx_up);
                }
                if x > 0 {
                    let (tx_left, rx_left) = mpsc::channel::<Packet>(BUFFER_SIZE);
                    cur_node.tx_left = Some(tx_left);

                    let left_node = rx_map.entry((x - 1, y)).or_default();
                    left_node.left = Some(rx_left);
                }
                if x < width {
                    let (tx_right, rx_right) = mpsc::channel::<Packet>(BUFFER_SIZE);
                    cur_node.tx_right = Some(tx_right);

                    let right_node = rx_map.entry((x + 1, y)).or_default();
                    right_node.right = Some(rx_right);
                }

                grid_row.push(Arc::new(cur_node));
            }

            temp_nodes.push(grid_row);
        }

        // Assign the rx channels
        //        for y in 0..height {
        //            for x in 0..width {
        //                let cur_node = &mut temp_nodes[y as usize][x as usize];
        //                if let Some(rx_conns) = rx_map.get_mut(&(x, y)) {
        //                    tokio::spawn(async move {});
        //
        //                    cur_node.rx_up = mem::take(&mut rx_conns.up);
        //                    cur_node.rx_down = mem::take(&mut rx_conns.down);
        //                    cur_node.rx_left = mem::take(&mut rx_conns.left);
        //                    cur_node.rx_right = mem::take(&mut rx_conns.right);
        //                }
        //            }
        //        }

        // Create the Arc MeshNode holder for thread safe access + optimized space usage
        let inner_nodes_arc: Vec<Arc<[Arc<MeshNode>]>> = temp_nodes
            .into_iter()
            .map(|node_row| Arc::from(node_row.into_boxed_slice()))
            .collect();
        self.nodes = Arc::from(inner_nodes_arc.into_boxed_slice());

        for y in 0..height {
            for x in 0..width {
                let cur_node = Arc::clone(&self.nodes[y as usize][x as usize]);
                let mut rx_up = None;
                let mut rx_down = None;
                let mut rx_left = None;
                let mut rx_right = None;

                if let Some(rx_conns) = rx_map.get_mut(&(x, y)) {
                    rx_up = mem::take(&mut rx_conns.up);
                    rx_down = mem::take(&mut rx_conns.down);
                    rx_left = mem::take(&mut rx_conns.left);
                    rx_right = mem::take(&mut rx_conns.right);
                }

                tokio::spawn(async move {
                    receive_packets(cur_node, rx_up, rx_down, rx_left, rx_right).await
                });
            }
        }
    }

    pub fn access_node(&self, (x, y): (u8, u8)) -> Result<Arc<MeshNode>, GridAccessError> {
        let node_row = self
            .nodes
            .get(y as usize)
            .ok_or_else(|| GridAccessError::InvalidHeight(x))?;
        let node = node_row
            .get(x as usize)
            .ok_or_else(|| GridAccessError::InvalidWidth(y))?;

        Ok(Arc::clone(node))
    }
}

// Takes a packet and sends it from a src node to a destination node
// Calculates the first direction and enquues in that mpsc tx, nodes carry from there
pub async fn send_packet_grid(
    mut packet: Packet,
    src_node: Arc<MeshNode>,
    dest: (u8, u8),
) -> Result<(), NodeCommError> {
    packet.header.destination = dest;
    tokio::spawn(async move {
        let result = send_packet(Arc::clone(&src_node), packet).await;
        if let Err(e) = result {
            println!("Error sending packet in spawned task: {:?}", e);
        }
    });

    Ok(())
}
