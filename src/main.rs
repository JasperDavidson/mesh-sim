use tokio::sync::mpsc::{Receiver, Sender};

enum Packet {
    Message(String),
    Integer(u64),
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

// Need some way to handle data transfer
struct MeshNode {
    process_buffer: Vec<Packet>,
    send_buffer: Vec<Packet>,
    tx_rate: u32,
    rx_rate: u32,

    // Sender and Receiver handles from/to each direction
    // Should probably be formed as Options since boundary nodes may lack them?
    tx_up: Sender<Packet>,
    tx_left: Sender<Packet>,
    tx_down: Sender<Packet>,
    tx_right: Sender<Packet>,

    rx_up: Receiver<Packet>,
    rx_left: Receiver<Packet>,
    rx_down: Receiver<Packet>,
    rx_right: Receiver<Packet>,
}

// We can be receiving data while processing/sending data - account for this
// Maybe a separate task waiting for data to come in and other processing data if available

// Thinking through this:
//  - There should be a separate async function that receives the packets as they come in and
//  emplaces them in the buffer
//      - How should we handle many packets coming in at once? Probably just a select! macro that
//      reads into the input buffer immediately then selects again
//  - We need a way to process this data and send it out, but imagine that the data processing at
//  each node could be computationally intensive
//  - Could launch a blocking thread to do the computation, then place the processed data into
//  another queue to be sent out
//      - Actually, this should depend on if the node is a Routing or Processing node
//      - A processing node can do everything a routing node can (redirect) but also process
//      packets in some way
impl MeshNode {
    fn calc_route(&self, dest: &MeshNode) -> Direction {}

    async fn receive_packets(&mut self) {}

    fn send_packet(&self, packet: Packet, direction: Direction) {}
}

// Should figure out how to align each MeshNode at 64 byte boundary to avoid false sharing
struct Grid {
    nodes: Vec<Vec<MeshNode>>,
}

impl Grid {
    // Need a robust way to build a grid of nodes of dimensions width x height
    // - How will we handle instantiating the channels?
    // - Maybe doing a transmit pass where we build all the channels and store the transmitters in
    // the structs and store the receivers in a map based on coordinate, then a receive pass that
    // checks the map for each node and provides the receivers
    fn new(width: u8, height: u8) -> Self {}
}

#[tokio::main]
async fn main() {}
