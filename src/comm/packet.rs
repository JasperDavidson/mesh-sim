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
    pub dir: Direction,
    pub destination: (u8, u8),
}

// Might not need to derive default here, look into deleting in the future
#[derive(Default, Debug)]
pub struct Packet {
    pub header: MetaData,
    data: PacketData,
}
