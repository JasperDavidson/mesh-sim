use std::time::Duration;

use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedSender};

use crate::comm::packet::{Event, Packet};

#[derive(Copy, Clone, PartialEq, Default, Debug)]
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
    SendErrorPacket(#[from] SendError<Packet>),
    #[error(
        "Channel unable to send events: Check corresponding Receiver is alive and the channel is open"
    )]
    SendErrorEvent(#[from] SendError<Event>),
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

pub fn calc_path(mut cur_pos: (u8, u8), dest_pos: (u8, u8)) -> Vec<Direction> {
    let mut x_delta = dest_pos.0 as i16 - cur_pos.0 as i16;
    let mut y_delta = dest_pos.1 as i16 - cur_pos.1 as i16;
    let mut path_vec = Vec::new();
    let mut neg_first_greedy;

    let mut chosen_path: Vec<Direction>;
    while !(x_delta == 0 && y_delta == 0) {
        if x_delta > 0 && y_delta > 0 {
            neg_first_greedy = Direction::Down;
        } else if x_delta > 0 && y_delta < 0 {
            neg_first_greedy = Direction::Right;
        } else if x_delta < 0 && y_delta > 0 {
            neg_first_greedy = Direction::Left;
        } else if x_delta < 0 && y_delta < 0 {
            neg_first_greedy = Direction::Left;
        } else if x_delta == 0 && y_delta < 0 {
            neg_first_greedy = Direction::Up;
        } else if x_delta == 0 && y_delta > 0 {
            neg_first_greedy = Direction::Down;
        } else if y_delta == 0 && x_delta < 0 {
            neg_first_greedy = Direction::Left;
        } else
        /* y_delta == 0 && x_delta > 0 */
        {
            neg_first_greedy = Direction::Right;
        }

        if neg_first_greedy == Direction::Left || neg_first_greedy == Direction::Right {
            chosen_path = vec![neg_first_greedy; x_delta.abs() as usize];
            x_delta = 0;
            cur_pos.0 = cur_pos.0 + x_delta as u8;
        } else {
            chosen_path = vec![neg_first_greedy; y_delta.abs() as usize];
            y_delta = 0;
            cur_pos.1 = cur_pos.1 + y_delta as u8;
        }

        path_vec.append(&mut chosen_path);
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
    inner_tx_up: &Sender<Packet>,
    inner_tx_down: &Sender<Packet>,
    inner_tx_left: &Sender<Packet>,
    inner_tx_right: &Sender<Packet>,
    event_tx: &UnboundedSender<Event>,
    tx_rate: u64,
) -> Result<(), NodeCommError> {
    loop {
        // let node_clone = Arc::clone(&node);
        tokio::time::sleep(std::time::Duration::from_millis((1 / tx_rate) * 1000)).await;
        select! {
            Some(mut packet) = async {
                if let Some(rx) = rx_up.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                packet.header.cur_pos = (packet.header.cur_pos.0, packet.header.cur_pos.1 + 1);
                if packet.header.path_step == packet.header.path.len() {
                        event_tx.send(Event::PacketArrived { id: packet.header.id, at: packet.header.cur_pos, dest: packet.header.dest_pos })?;
                        continue;
                }
                event_tx.send(Event::PacketReceived { id: packet.header.id, recv_dir: Direction::Up, at: packet.header.cur_pos })?;
                inner_tx_up.send(packet).await?;
            },
            Some(mut packet) = async {
                if let Some(rx) = rx_down.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                packet.header.cur_pos = (packet.header.cur_pos.0, packet.header.cur_pos.1 - 1);
                if packet.header.path_step == packet.header.path.len() {
                        event_tx.send(Event::PacketArrived { id: packet.header.id, at: packet.header.cur_pos, dest: packet.header.dest_pos })?;
                        continue;
                }
                event_tx.send(Event::PacketReceived { id: packet.header.id, recv_dir: Direction::Down, at: packet.header.cur_pos })?;
                inner_tx_down.send(packet).await?;
            },
            Some(mut packet) = async {
                if let Some(rx) = rx_left.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                packet.header.cur_pos = (packet.header.cur_pos.0 + 1, packet.header.cur_pos.1);
                if packet.header.path_step == packet.header.path.len() {
                        event_tx.send(Event::PacketArrived { id: packet.header.id, at: packet.header.cur_pos, dest: packet.header.dest_pos })?;
                        continue;
                }
                event_tx.send(Event::PacketReceived { id: packet.header.id, recv_dir: Direction::Left, at: packet.header.cur_pos })?;
                inner_tx_left.send(packet).await?;
            },
            Some(mut packet) = async {
                if let Some(rx) = rx_right.as_mut() {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                packet.header.cur_pos = (packet.header.cur_pos.0 - 1, packet.header.cur_pos.1);
                if packet.header.path_step == packet.header.path.len() {
                        event_tx.send(Event::PacketArrived { id: packet.header.id, at: packet.header.cur_pos, dest: packet.header.dest_pos })?;
                        continue;
                }
                event_tx.send(Event::PacketReceived { id: packet.header.id, recv_dir: Direction::Right, at: packet.header.cur_pos })?;
                inner_tx_right.send(packet).await?;
            },
        }
    }
}

// This should be async so that a task can be spawned with the purpose enqueing the packet
// - Might need to await while channel is being processed
pub async fn send_packet(
    // node: Arc<MeshNode>,
    // mut packet: Packet,
    tx_up: Option<&Sender<Packet>>,
    tx_down: Option<&Sender<Packet>>,
    tx_left: Option<&Sender<Packet>>,
    tx_right: Option<&Sender<Packet>>,
    tx_event: &UnboundedSender<Event>,
    mut inner_rx_up: Receiver<Packet>,
    mut inner_rx_down: Receiver<Packet>,
    mut inner_rx_left: Receiver<Packet>,
    mut inner_rx_right: Receiver<Packet>,
    mut inner_rx_local: Receiver<Packet>,
    rx_rate: u64,
) -> Result<(), NodeCommError> {
    loop {
        tokio::time::sleep(Duration::from_millis((1 / rx_rate) * 1000)).await;
        select! {
            Some(inner_packet) = inner_rx_up.recv() => {
                transmit_dir(inner_packet, tx_up, tx_down, tx_left, tx_right, &tx_event).await?
            }
            Some(inner_packet) = inner_rx_down.recv() => {
                transmit_dir(inner_packet, tx_up, tx_down, tx_left, tx_right, &tx_event).await?
            }
            Some(inner_packet) = inner_rx_left.recv() => {
                transmit_dir(inner_packet, tx_up, tx_down, tx_left, tx_right, &tx_event).await?
            }
            Some(inner_packet) = inner_rx_right.recv() => {
                transmit_dir(inner_packet, tx_up, tx_down, tx_left, tx_right, &tx_event).await?
            },
            Some(inner_packet) = inner_rx_local.recv() => {
                transmit_dir(inner_packet, tx_up, tx_down, tx_left, tx_right, &tx_event).await?
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
    tx_event: &UnboundedSender<Event>,
) -> Result<(), NodeCommError> {
    packet.header.dir = packet.header.path[packet.header.path_step];
    packet.header.path_step += 1;

    match packet.header.dir {
        Direction::Up => {
            tx_event.send(Event::PacketSent {
                id: packet.header.id,
                send_dir: Direction::Up,
                from: packet.header.cur_pos,
            })?;
            tx_up
                .expect("Up direction should've been supported")
                .send(packet)
                .await?
        }
        Direction::Down => {
            tx_event.send(Event::PacketSent {
                id: packet.header.id,
                send_dir: Direction::Down,
                from: packet.header.cur_pos,
            })?;
            tx_down
                .expect("Down direction should've been supported")
                .send(packet)
                .await?
        }
        Direction::Left => {
            tx_event.send(Event::PacketSent {
                id: packet.header.id,
                send_dir: Direction::Left,
                from: packet.header.cur_pos,
            })?;
            tx_left
                .expect("Left direction should've been supported")
                .send(packet)
                .await?
        }
        Direction::Right => {
            tx_event.send(Event::PacketSent {
                id: packet.header.id,
                send_dir: Direction::Right,
                from: packet.header.cur_pos,
            })?;
            tx_right
                .expect("Right direction should've been supported")
                .send(packet)
                .await?
        }
        _ => unreachable!(),
    }

    Ok(())
}
