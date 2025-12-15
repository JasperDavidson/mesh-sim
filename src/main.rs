mod arch;
use crate::arch::grid::Grid;

#[tokio::main]
async fn main() {
    let mut grid: Grid = Grid::default();
    grid.init_grid(5, 5);
}
