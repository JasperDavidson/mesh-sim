use crate::comm::transfer::Direction;

pub enum Event {
    PacketArrived {
        id: usize,
        at: (u8, u8),
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
enum PacketData {
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
    pub destination: (u8, u8),
}

// Might not need to derive default here, look into deleting in the future
#[derive(Default, Debug)]
pub struct Packet {
    pub header: MetaData,
    data: PacketData,
}

//impl Packet {
//    pub fn new(data: u64) -> Self {
//        let metadata = MetaData { id:  };
//    }
//}
