mod arch;
mod comm;
use crate::{arch::grid::Grid, comm::packet::Packet};

#[tokio::main]
async fn main() {
    let mut grid: Grid = Grid::default();
    grid.init_grid(5, 5);

    let packet = Packet::default();
    if let Ok(src_node) = grid.access_node((4, 3)) {
        Grid::send_packet_grid(src_node, packet, (1, 0))
            .await
            .expect("Failed to send packet");
    }

    tokio::signal::ctrl_c().await.unwrap();
}
