use crate::arch::node::MeshNode;
use crate::comm::packet::Packet;
use std::sync::Arc;
use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::error::SendError;

#[derive(Copy, Clone, Default, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    #[default]
    Init,
}

#[derive(Error, Debug)]
pub enum SendDirError {
    #[error("Tried to send a packet out of bounds (y value below 0)")]
    Up,
    #[error("Tried to send a packet out of bounds (y value greater than height of mesh)")]
    Down,
    #[error("Tried to send a packet out of bounds (x value less than 0)")]
    Left,
    #[error("Tried to send a packet out of bounds (x value greater than width of mesh)")]
    Right,
}

#[derive(Error, Debug)]
pub enum NodeCommError {
    // Make sure this zero actually passes the error string defined above
    #[error("{0}")]
    SendDirError(SendDirError),
    #[error(
        "Channel unable to send packets: Check corresponding Receiver is alive and the channel is open"
    )]
    SendError(#[from] SendError<Packet>),
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
    // Calculating the two best greediest moves, second best is for if one is invalidated due to turn
    // restriction routing
    // Turn restriction routing accounts for most edge cases, unless the node is on the edge
    // Need to account for reaching the edges of the grid
    pub fn greedy_move(src_pos: (u8, u8), dest_pos: (u8, u8)) -> [Direction; 2] {
        let x_delta = dest_pos.0 as i16 - src_pos.0 as i16;
        let y_delta = dest_pos.1 as i16 - src_pos.1 as i16;

        println!("{x_delta}, {y_delta}");
        if (x_delta == 0) && (y_delta == 0) {
            panic!("Target node reached");
        }

        if x_delta > 0 && y_delta > 0 {
            [Direction::Down, Direction::Right]
        } else if x_delta > 0 && y_delta < 0 {
            [Direction::Right, Direction::Up]
        } else if x_delta < 0 && y_delta > 0 {
            [Direction::Left, Direction::Down]
        } else if x_delta < 0 && y_delta < 0 {
            [Direction::Left, Direction::Up]
        } else if x_delta == 0 && y_delta < 0 {
            [Direction::Up, Direction::Down]
        } else if x_delta == 0 && y_delta > 0 {
            [Direction::Down, Direction::Up]
        } else if y_delta == 0 && x_delta < 0 {
            [Direction::Left, Direction::Right]
        } else
        /* y_delta == 0 && x_delta > 0 */
        {
            [Direction::Right, Direction::Left]
        }
    }

    // Initially will implement simple turn restriction routing to prevent deadlocking
    //  - For turn restriction needs to know the prev. direction to detect banned turns
    //  - Uses Negative First Routing, so Left->Down and Up->Left are disallowed paths
    // Can expand do adaptive routing later
    fn calc_route(&self, dest: (u8, u8), prev_dir: &Direction) -> Direction {
        let greedy_preferences = MeshNode::greedy_move((self.x, self.y), (dest.0, dest.1));
        println!("{:?}", greedy_preferences);

        if matches!(
            (prev_dir, &greedy_preferences[0]),
            (Direction::Up, Direction::Left) | (Direction::Right, Direction::Down)
        ) {
            println!("matched unallowed turn");
            println!("{:?}, {:?}", prev_dir, &greedy_preferences[0]);
            greedy_preferences[1]
        } else {
            greedy_preferences[0]
        }
    }
}

// The flow for the send/receive packet functions should be:
// - select! selects a channel to read a packet from via the rx await from cardinal directions
// - A receive_packet() task is launched to process the packet and redirect it
//  - This at some point launches a send_packet() task to enqueue it in a new channel

// This should be async so that a task can be spawned with the purpose of routing a packet

// Instead of MeshNode objects owning their Receivers, should receive_packets take in the
// Receivers and constantly spin as as a tokio task?
pub async fn receive_packets(
    node: Arc<MeshNode>,
    mut rx_up: Option<Receiver<Packet>>,
    mut rx_down: Option<Receiver<Packet>>,
    mut rx_left: Option<Receiver<Packet>>,
    mut rx_right: Option<Receiver<Packet>>,
) {
    loop {
        let node_clone = Arc::clone(&node);
        select! {
            Some(packet) = async {
                if let Some(rx) = rx_up.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                println!("Receiving from up!");
                tokio::spawn(async move { send_packet(node_clone, packet).await });
            },
            Some(packet) = async {
                if let Some(rx) = rx_down.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                println!("Receiving from down!");
                tokio::spawn(async move { send_packet(node_clone, packet).await });
            },
            Some(packet) = async {
                if let Some(rx) = rx_left.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                println!("Receiving from left!");
                tokio::spawn(async move { send_packet(node_clone, packet).await });
            },
            Some(packet) = async {
                if let Some(rx) = rx_right.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                println!("Receiving from right!");
                tokio::spawn(async move { send_packet(node_clone, packet).await });
            },
        }
    }
}

// This should be async so that a task can be spawned with the purpose enqueing the packet
// - Might need to await while channel is being processed
pub async fn send_packet(node: Arc<MeshNode>, mut packet: Packet) -> Result<(), NodeCommError> {
    packet.header.dir = node.calc_route(
        (packet.header.destination.0, packet.header.destination.1),
        &packet.header.dir,
    );

    // TODO Look into error propagation handling in async task
    match packet.header.dir {
        Direction::Up => {
            println!("Heading up");
            node.tx_up
                .as_ref()
                .ok_or_else(|| NodeCommError::SendDirError(SendDirError::Up))?
                .send(packet)
                .await?
        }
        Direction::Down => {
            println!("Heading down");
            node.tx_down
                .as_ref()
                .ok_or_else(|| NodeCommError::SendDirError(SendDirError::Down))?
                .send(packet)
                .await?
        }
        Direction::Left => {
            println!("Heading left");
            node.tx_left
                .as_ref()
                .ok_or_else(|| NodeCommError::SendDirError(SendDirError::Left))?
                .send(packet)
                .await?
        }
        Direction::Right => {
            println!("Heading right");
            let tx = node
                .tx_right
                .as_ref()
                .ok_or_else(|| NodeCommError::SendDirError(SendDirError::Right))?;

            // Handle the result explicitly to see the error
            match tx.send(packet).await {
                Ok(_) => println!("Packet sent!"),
                Err(e) => println!("Failed to send packet: {:?}", e),
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
