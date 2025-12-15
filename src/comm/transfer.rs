use crate::arch::node::MeshNode;
use crate::comm::packet::Packet;

enum Direction {
    Up,
    Down,
    Left,
    Right,
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
//  - Could launch a blocking thread to do the computation
//      - Actually, this should depend on if the node is a Routing or Processing node
//      - A processing node can do everything a routing node can (redirect) but also process
//      packets in some way
//      - For now we'll simulate it as if every node in the middle is simply routing, but a V2
//      might simulate both router and cpu running at the same time -> local channel?
impl MeshNode {
    //    fn calc_route(&self, dest: &MeshNode) -> Direction {}

    async fn receive_packets(&mut self) {}

    fn send_packet(&self, packet: Packet, direction: Direction) {}
}
