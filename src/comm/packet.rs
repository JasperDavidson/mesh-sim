enum PacketData {
    Message(String),
    Integer(u64),
}

struct MetaData {
    destination: (u64, u64),
}

pub struct Packet {
    header: MetaData,
    data: PacketData,
}
