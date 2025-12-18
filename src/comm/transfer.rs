use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::arch::node::MeshNode;
use crate::comm::packet::Packet;

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
        print!("{:?}", greedy_preferences);

        if matches!(
            (prev_dir, &greedy_preferences[0]),
            (Direction::Up, Direction::Left) | (Direction::Right, Direction::Down)
        ) {
            greedy_preferences[1]
        } else {
            greedy_preferences[0]
        }
    }
}

fn turn_restrict(
    path_vec: &mut Vec<Direction>,
    greedy_preferences: &[Direction; 2],
    prev_dir: &Direction,
    cur_pos: &mut (u8, u8),
) {
    let dir_chosen;

    if matches!(
        (prev_dir, &greedy_preferences[0]),
        (Direction::Up, Direction::Left) | (Direction::Right, Direction::Down)
    ) {
        dir_chosen = greedy_preferences[1];
        path_vec.push(dir_chosen);
    } else {
        dir_chosen = greedy_preferences[0];
        path_vec.push(dir_chosen);
    }

    match dir_chosen {
        Direction::Up => cur_pos.1 -= 1,
        Direction::Down => cur_pos.1 += 1,
        Direction::Left => cur_pos.0 -= 1,
        Direction::Right => cur_pos.0 += 1,
        Direction::Init => unreachable!(),
    }
}

pub fn calc_path(mut cur_pos: (u8, u8), dest_pos: (u8, u8)) -> Vec<Direction> {
    let mut x_delta = dest_pos.0 as i16 - cur_pos.0 as i16;
    let mut y_delta = dest_pos.1 as i16 - cur_pos.1 as i16;
    let mut path_vec = Vec::new();
    let mut greedy_preferences;
    let prev_dir = Direction::Init;

    while !(x_delta == 0 && y_delta == 0) {
        if x_delta > 0 && y_delta > 0 {
            greedy_preferences = [Direction::Down, Direction::Right];
        } else if x_delta > 0 && y_delta < 0 {
            greedy_preferences = [Direction::Right, Direction::Up];
        } else if x_delta < 0 && y_delta > 0 {
            greedy_preferences = [Direction::Left, Direction::Down];
        } else if x_delta < 0 && y_delta < 0 {
            greedy_preferences = [Direction::Left, Direction::Up];
        } else if x_delta == 0 && y_delta < 0 {
            greedy_preferences = [Direction::Up, Direction::Down];
        } else if x_delta == 0 && y_delta > 0 {
            greedy_preferences = [Direction::Down, Direction::Up];
        } else if y_delta == 0 && x_delta < 0 {
            greedy_preferences = [Direction::Left, Direction::Right];
        } else
        /* y_delta == 0 && x_delta > 0 */
        {
            greedy_preferences = [Direction::Right, Direction::Left];
        }

        turn_restrict(&mut path_vec, &greedy_preferences, &prev_dir, &mut cur_pos);
        x_delta = dest_pos.0 as i16 - cur_pos.0 as i16;
        y_delta = dest_pos.1 as i16 - cur_pos.1 as i16;
    }

    path_vec
}

// The flow for the send/receive packet functions should be:
// - select! selects a channel to read a packet from via the rx await from cardinal directions
// - A receive_packet() task is launched to process the packet and redirect it
//  - This at some point launches a send_packet() task to enqueue it in a new channel

// This should be async so that a task can be spawned with the purpose of routing a packet

// Instead of MeshNode objects owning their Receivers, should receive_packets take in the
// Receivers and constantly spin as as a tokio task?
pub async fn receive_packets(
    // node: Arc<MeshNode>,
    mut rx_up: Option<Receiver<Packet>>,
    mut rx_down: Option<Receiver<Packet>>,
    mut rx_left: Option<Receiver<Packet>>,
    mut rx_right: Option<Receiver<Packet>>,
    inner_tx_up: Sender<Packet>,
    inner_tx_down: Sender<Packet>,
    inner_tx_left: Sender<Packet>,
    inner_tx_right: Sender<Packet>,
) -> Result<(), NodeCommError> {
    loop {
        // let node_clone = Arc::clone(&node);
        select! {
            Some(packet) = async {
                if let Some(rx) = rx_up.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                println!(" -> Receiving from up!\n");
                if packet.header.path_step == packet.header.path.len() {
                        panic!("Packet reached destination");
                }
                inner_tx_up.send(packet).await?;
                // tokio::spawn(async move { send_packet(node_clone, packet).await });
            },
            Some(packet) = async {
                if let Some(rx) = rx_down.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                println!(" -> Receiving from down!\n");
                if packet.header.path_step == packet.header.path.len() {
                        panic!("Packet reached destination");
                }
                inner_tx_down.send(packet).await?;
                // tokio::spawn(async move { send_packet(node_clone, packet).await });
            },
            Some(packet) = async {
                if let Some(rx) = rx_left.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                println!(" -> Receiving from left!\n");
                if packet.header.path_step == packet.header.path.len() {
                        panic!("Packet reached destination");
                }
                inner_tx_left.send(packet).await?;
                // tokio::spawn(async move { send_packet(node_clone, packet).await });
            },
            Some(packet) = async {
                if let Some(rx) = rx_right.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                println!(" -> Receiving from right!\n");
                if packet.header.path_step == packet.header.path.len() {
                        panic!("Packet reached destination");
                }
                inner_tx_right.send(packet).await?;
                // tokio::spawn(async move { send_packet(node_clone, packet).await });
            },
        }
    }
}

// This should be async so that a task can be spawned with the purpose enqueing the packet
// - Might need to await while channel is being processed
pub async fn send_packet(
    // node: Arc<MeshNode>,
    // mut packet: Packet,
    tx_up: Option<Sender<Packet>>,
    tx_down: Option<Sender<Packet>>,
    tx_left: Option<Sender<Packet>>,
    tx_right: Option<Sender<Packet>>,
    mut inner_rx_up: Receiver<Packet>,
    mut inner_rx_down: Receiver<Packet>,
    mut inner_rx_left: Receiver<Packet>,
    mut inner_rx_right: Receiver<Packet>,
    mut inner_rx_local: Receiver<Packet>,
) -> Result<(), NodeCommError> {
    loop {
        select! {
            Some(inner_packet) = inner_rx_up.recv() => {
                transmit_dir(inner_packet, tx_up.as_ref(), tx_down.as_ref(), tx_left.as_ref(), tx_right.as_ref()).await?
            }
            Some(inner_packet) = inner_rx_down.recv() => {
                transmit_dir(inner_packet, tx_up.as_ref(), tx_down.as_ref(), tx_left.as_ref(), tx_right.as_ref()).await?
            }
            Some(inner_packet) = inner_rx_left.recv() => {
                transmit_dir(inner_packet, tx_up.as_ref(), tx_down.as_ref(), tx_left.as_ref(), tx_right.as_ref()).await?
            }
            Some(inner_packet) = inner_rx_right.recv() => {
                transmit_dir(inner_packet, tx_up.as_ref(), tx_down.as_ref(), tx_left.as_ref(), tx_right.as_ref()).await?
            },
            Some(inner_packet) = inner_rx_local.recv() => {
                transmit_dir(inner_packet, tx_up.as_ref(), tx_down.as_ref(), tx_left.as_ref(), tx_right.as_ref()).await?
            }
        }
    }
}

async fn transmit_dir(
    mut packet: Packet,
    tx_up: Option<&Sender<Packet>>,
    tx_down: Option<&Sender<Packet>>,
    tx_left: Option<&Sender<Packet>>,
    tx_right: Option<&Sender<Packet>>,
) -> Result<(), NodeCommError> {
    packet.header.dir = packet.header.path[packet.header.path_step];
    packet.header.path_step += 1;

    match packet.header.dir {
        Direction::Up => {
            print!(" -> Chose up");
            tx_up
                .expect("Up direction should've been supported")
                .send(packet)
                .await?
        }
        Direction::Down => {
            print!(" -> Chose down");
            tx_down
                .expect("Down direction should've been supported")
                .send(packet)
                .await?
        }
        Direction::Left => {
            print!(" -> Chose left");
            tx_left
                .expect("Left direction should've been supported")
                .send(packet)
                .await?
        }
        Direction::Right => {
            print!(" -> Chose right");
            tx_right
                .expect("Right direction should've been supported")
                .send(packet)
                .await?
        }
        _ => unreachable!(),
    }

    Ok(())
}
