mod arch;
mod comm;
use crate::{
    arch::grid::Grid,
    comm::packet::{Event, Packet},
};

#[tokio::main]
async fn main() {
    let mut grid: Grid = Grid::default();
    let mut event_rx = grid.init_grid(5, 5);

    let mut packet = Packet::default();
    packet.header.cur_pos = (4, 3);
    if let Ok(src_node) = grid.access_node((4, 3)) {
        Grid::send_packet_grid(src_node, packet, (1, 0))
            .await
            .expect("Failed to send packet");
    }

    loop {
        if let Some(event) = event_rx.recv().await {
            match event {
                Event::PacketArrived { id, at } => {
                    println!("Packet id {id} arrived at {:?}!", at);
                }
                Event::PacketReceived { id, recv_dir, at } => {
                    println!("Packet id {id} received at {:?} from {:?}", at, recv_dir);
                }
                Event::PacketSent { id, send_dir, from } => {
                    println!("Packet id {id} sent {:?} from {:?}", send_dir, from);
                }
            }
        }
    }

    // tokio::signal::ctrl_c().await.unwrap();
}
