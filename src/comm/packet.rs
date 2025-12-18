use std::sync::atomic::{AtomicUsize, Ordering};

use crate::comm::transfer::{Direction, calc_path};

pub enum Event {
    PacketArrived {
        id: usize,
        at: (u8, u8),
        dest: (u8, u8),
    },
    PacketReceived {
        id: usize,
        recv_dir: Direction,
        at: (u8, u8),
    },
    PacketSent {
        id: usize,
        send_dir: Direction,
        from: (u8, u8),
    },
}

#[derive(Default, Debug)]
pub enum PacketData {
    Message(String),
    Integer(u64),
    #[default]
    Default,
}

#[derive(Default, Debug)]
pub struct MetaData {
    pub id: usize,
    pub dir: Direction,
    pub path: Vec<Direction>,
    pub path_step: usize,
    pub cur_pos: (u8, u8),
    pub dest_pos: (u8, u8),
}

// Might not need to derive default here, look into deleting in the future
#[derive(Default, Debug)]
pub struct Packet {
    pub header: MetaData,
    data: PacketData,
}

impl Packet {
    fn get_id() -> usize {
        static PACKET_COUNTER: AtomicUsize = AtomicUsize::new(0);
        PACKET_COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    pub fn new(data: PacketData, src_pos: (u8, u8), dest_pos: (u8, u8)) -> Self {
        let header = MetaData {
            id: Packet::get_id(),
            dir: Direction::Init,
            path: Vec::new(),
            path_step: 0,
            cur_pos: src_pos,
            dest_pos,
        };

        Self { header, data }
    }
}
