mod arch;
mod comm;
use crate::{
    arch::grid::Grid,
    arch::grid::send_packet_grid,
    comm::packet::{self, Packet},
};
use std::sync::Arc;
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() {
    let mut grid: Grid = Grid::default();
    grid.init_grid(5, 5);

    let packet = Packet::default();
    if let Ok(src_node) = grid.access_node((2, 4)) {
        send_packet_grid(packet, Arc::clone(&src_node), (1, 0))
            .await
            .expect("Failed to send packet");
    }

    sleep(Duration::from_micros(1)).await;
    // tokio::signal::ctrl_c().await.unwrap();
}
