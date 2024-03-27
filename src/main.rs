use macroquad::prelude::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cell {
    Dead = 0,
    Alive = 1,
}

struct World {
    width: isize,
    height: isize,
    cells: Vec<Cell>,
}
static ALIVE_PROPORTION: f32 = 0.0;

impl World {
    fn new(width: isize, height: isize) -> World {
        World {
            width,
            height,
            cells: vec![Cell::Dead; (width * height) as usize],
        }
    }

    fn randomize(&mut self) {
        for cell in self.cells.iter_mut() {
            *cell = if rand::rand() < ((u32::MAX as f32) * ALIVE_PROPORTION) as u32 {
                Cell::Alive
            } else {
                Cell::Dead
            };
        }
    }

    fn get_index(&self, x: isize, y: isize) -> usize {
        let x = if x < 0 { self.width + x } else { x };
        let y = if y < 0 { self.height + y } else { y };
        (y* self.width + x) as usize
    }

    fn get_cell(&self, x: isize, y: isize) -> Cell {
        self.cells[self.get_index(x, y)]
    }

    fn set_cell(&mut self, x: isize, y: isize, cell: Cell) {
        let index = self.get_index(x, y);
        self.cells[index] = cell;
    }

    fn get_neighbour_count(&self, x: isize, y: isize) -> u8 {
        let mut count = 0;
        for dx in 0isize..3 {
            for dy in 0isize..3 {
                if dx == 1 && dy == 1 {
                    continue;
                }
                let nx = x + dx - 1;
                let ny = y + dy - 1;
                if nx < self.width && ny < self.height {
                    count += self.get_cell(nx, ny) as u8;
                }
            }
        }
        count
    }

    fn next_generation(&self) -> World {
        let mut next = World::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let count = self.get_neighbour_count(x, y);
                let cell = self.get_cell(x, y);
                let next_cell = match (cell, count) {
                    (Cell::Alive, 2) | (Cell::Alive, 3) => Cell::Alive,
                    (Cell::Dead, 3) => Cell::Alive,
                    _ => Cell::Dead,
                };
                next.set_cell(x, y, next_cell);
            }
        }
        next
    }

    fn spawn_group(&mut self, x: isize, y: isize) {
        // 5x5 around point
        for dx in 0..5 {
            for dy in 0..5 {
                let nx = x + dx - 1;
                let ny = y + dy - 1;
                if nx < self.width && ny < self.height {
                    // 1 in 3 chance of spawning a cell
                    if rand::rand() < (u32::MAX / 3) {
                        self.set_cell(nx, ny, Cell::Alive);
                    }
                }
            }
        }
    }
}

// slate-900
// static WORLD_COLOUR: Color = color_u8!(15, 23, 42, 0);
// static WORLD_COLOUR: Color = color_u8!(255, 255, 255, 0);
static WORLD_COLOUR: Color = color_u8!(0, 0, 0, 0);
static GRID_SIZE: usize = 10;
static CELL_SIZE: usize = 8;
static OFFSET: usize = (GRID_SIZE - CELL_SIZE) / 2;
// cyan-600
static CELL_COLOUR: Color = color_u8!(8, 145, 178, 64);

static FRAME_TIME: f32 = 1.0 / 6.0;
static SPAWN_TIME: f32 = 1.0;

#[macroquad::main("Game of Life")]
async fn main() {

    // Pseuo-random seed generator
    let time = (get_time() * 100_000.0).powi(3) as u64;
    rand::srand(time);

    let width = (screen_width() as usize / GRID_SIZE) as isize;
    let height = (screen_height() as usize / GRID_SIZE) as isize;
    let mut world = World::new(width, height);
    world.randomize();

    let mut elapsed_frame: f32 = 0.0;
    let mut elapsed_spawn: f32 = 0.0;

    loop {
        elapsed_frame += get_frame_time();
        if elapsed_frame > FRAME_TIME {
            elapsed_frame = 0.0;

            // Only update the world if the game is an 'update frame'
            world = world.next_generation();

            // Interactivity: click to add cells in a 5x5 square around the click
            if is_mouse_button_down(MouseButton::Left) {
                let x = (mouse_position().0 / GRID_SIZE as f32) as isize;
                let y = (mouse_position().1 / GRID_SIZE as f32) as isize;

                world.spawn_group(x, y);
            }
        }

        // Spawn some random cells
        elapsed_spawn += get_frame_time();
        if elapsed_spawn > SPAWN_TIME {
            elapsed_spawn = 0.0;
            let x = rand::rand() as isize % width;
            let y = rand::rand() as isize % height;
            world.spawn_group(x, y);
        }
        
        // Clear the frame
        clear_background(WORLD_COLOUR);

        // Render the world
        for y in 0..world.height {
            for x in 0..world.width {
                let cell = world.get_cell(x, y);
                if cell == Cell::Alive {
                    draw_rectangle(
                        x as f32 * GRID_SIZE as f32 + OFFSET as f32,
                        y as f32 * GRID_SIZE as f32 + OFFSET as f32,
                        CELL_SIZE as f32,
                        CELL_SIZE as f32,
                        CELL_COLOUR,
                    );
                }
            }
        }
        
        // Get next frame
        next_frame().await
    }
}
