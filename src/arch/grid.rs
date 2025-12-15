use std::collections::HashMap;
use std::mem;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

use crate::arch::node::MeshNode;
use crate::comm::packet::Packet;

const BUFFER_SIZE: usize = 32;

// Should figure out how to align each MeshNode at 64 byte boundary to avoid false sharing
#[derive(Default)]
pub struct Grid {
    nodes: Vec<Vec<MeshNode>>,
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

        for y in 0..height {
            let mut grid_row = Vec::new();

            for x in 0..width {
                let mut cur_node = MeshNode::init_channeless(10, 10);

                // Create channels based on position
                if y < height {
                    let (tx_up, rx_up) = mpsc::channel::<Packet>(BUFFER_SIZE);
                    cur_node.tx_up = Some(tx_up);

                    let up_node = rx_map.entry((x, y - 1)).or_default();
                    up_node.up = Some(rx_up);
                }
                if y > 0 {
                    let (tx_down, rx_down) = mpsc::channel::<Packet>(BUFFER_SIZE);
                    cur_node.tx_down = Some(tx_down);

                    let down_node = rx_map.entry((x, y + 1)).or_default();
                    down_node.up = Some(rx_down);
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

                grid_row.push(cur_node);
            }

            self.nodes.push(grid_row);
        }

        // Assign the rx channels
        for y in 0..height {
            for x in 0..width {
                let cur_node = &mut self.nodes[y as usize][x as usize];
                if let Some(rx_conns) = rx_map.get_mut(&(x, y)) {
                    cur_node.rx_up = mem::take(&mut rx_conns.up);
                    cur_node.rx_down = mem::take(&mut rx_conns.down);
                    cur_node.rx_left = mem::take(&mut rx_conns.left);
                    cur_node.rx_right = mem::take(&mut rx_conns.right);
                }
            }
        }
    }
}
