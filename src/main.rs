mod arch;
mod comm;
use crate::{
    arch::grid::{Grid, GridAccessError},
    comm::packet::{Event, Packet, PacketData},
};

async fn send_packet(grid: &Grid, packet: Packet) {
    if let Ok(src_node) = grid.access_node(packet.header.cur_pos) {
        Grid::send_packet_grid(src_node, packet)
            .await
            .expect("Failed to send packet");
    }
}

#[tokio::main]
async fn main() {}

#[tokio::test]
async fn small_packet_load() -> Result<(), GridAccessError> {
    let mut grid: Grid = Grid::default();
    let mut event_rx = grid.init_grid(5, 5)?;

    let (src_1, dest_1) = ((4, 3), (1, 0));
    let (src_2, dest_2) = ((0, 0), (4, 4));
    let (src_3, dest_3) = ((1, 3), (4, 0));
    let (src_4, dest_4) = ((0, 0), (1, 1));
    let packet1 = Packet::new(PacketData::Integer(0), src_1, dest_1);
    let packet2 = Packet::new(PacketData::Integer(0), src_2, dest_2);
    let packet3 = Packet::new(PacketData::Integer(0), src_3, dest_3);
    let packet4 = Packet::new(PacketData::Integer(0), src_4, dest_4);

    let mut expected_arrived = 4;
    let test_result = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        send_packet(&grid, packet1).await;
        send_packet(&grid, packet2).await;
        send_packet(&grid, packet3).await;
        send_packet(&grid, packet4).await;

        while expected_arrived > 0 {
            if let Some(event) = event_rx.recv().await {
                match event {
                    // Only care about when packets arrive at their final destination
                    Event::PacketArrived { id, at, dest } => {
                        assert_eq!(
                            at, dest,
                            "Packet id {id} arrived at {:?} but should've arrived at {:?}",
                            at, dest
                        );
                        expected_arrived -= 1;
                    }
                    _ => {}
                }
            }
        }
    })
    .await;

    assert!(test_result.is_ok(), "Packet test timed out!");
    Ok(())
}

#[tokio::test]
async fn same_path_load() -> Result<(), GridAccessError> {
    let mut grid: Grid = Grid::default();
    let mut event_rx = grid.init_grid(5, 5)?;

    let (src, dest) = ((0, 0), (4, 4));
    let mut expected_arrived = 10000;

    let test_result = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        for _ in 0..expected_arrived {
            let packet = Packet::new(PacketData::Integer(0), src, dest);
            send_packet(&grid, packet).await;
        }

        while expected_arrived > 0 {
            if let Some(event) = event_rx.recv().await {
                match event {
                    // Only care about when packets arrive at their final destination
                    Event::PacketArrived { id, at, dest } => {
                        assert_eq!(
                            at, dest,
                            "Packet id {id} arrived at {:?} but should've arrived at {:?}",
                            at, dest
                        );
                        expected_arrived -= 1;
                    }
                    _ => {}
                }
            }
        }
    })
    .await;

    assert!(test_result.is_ok(), "Packet test timed out!");
    Ok(())
}
