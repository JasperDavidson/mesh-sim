use crate::comm::transfer::Direction;

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
