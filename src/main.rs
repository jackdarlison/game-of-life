use std::collections::HashMap;

use macroquad::{
    prelude::*,
    ui::{
        hash, root_ui,
        widgets::{Button, Window},
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Cell {
    state: u8,
}

impl Cell {
    fn dead() -> Self {
        Self { state: 0 }
    }

    fn alive() -> Self {
        Self { state: 1 }
    }
}

impl From<u8> for Cell {
    fn from(value: u8) -> Self {
        Cell { state: value }
    }
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
            cells: vec![Cell::dead(); (width * height) as usize],
        }
    }

    fn reset(&mut self) {
        self.cells = vec![Cell::dead(); (self.width * self.height) as usize];
    }

    fn randomize(&mut self) {
        for cell in self.cells.iter_mut() {
            *cell = if rand::rand() < ((u32::MAX as f32) * ALIVE_PROPORTION) as u32 {
                Cell::alive()
            } else {
                Cell::dead()
            };
        }
    }

    fn get_index(&self, x: isize, y: isize) -> usize {
        let x = if x < 0 { self.width + x } else { x };
        let y = if y < 0 { self.height + y } else { y };
        (y * self.width + x) as usize
    }

    fn get_cell(&self, x: isize, y: isize) -> Cell {
        self.cells[self.get_index(x, y)]
    }

    fn set_cell(&mut self, x: isize, y: isize, cell: Cell) {
        let index = self.get_index(x, y);
        self.cells[index] = cell;
    }

    fn get_neighbour_count(&self, x: isize, y: isize, include_self: bool) -> u8 {
        let mut count = 0;
        for dx in 0isize..3 {
            for dy in 0isize..3 {
                if !include_self && dx == 1 && dy == 1 {
                    continue;
                }
                let nx = x + dx - 1;
                let ny = y + dy - 1;
                if nx < self.width && ny < self.height {
                    let cell = self.get_cell(nx, ny);
                    if cell.state >= 1 {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    fn next_generation(&self, state_machine: RuleSetStateMachine) -> World {
        let mut next = World::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let count = self.get_neighbour_count(x, y, false);
                let cell = self.get_cell(x, y);

                let next_cell: Cell = state_machine
                    .get(&cell.state)
                    .and_then(|rules| {
                        rules
                            .get(&count)
                            .and_then(|v| Some(Cell { state: *v }))
                            .or(Some(cell))
                    })
                    .unwrap_or(cell);

                next.set_cell(x, y, next_cell);
            }
        }
        next
    }

    fn spawn_group(&mut self, x: isize, y: isize, size: isize) {
        if size == 0 {
            return;
        } else if size == 1 {
            self.set_cell(x, y, Cell::alive());
            return;
        }

        for dx in 0..size {
            for dy in 0..size {
                let nx = x + dx - 1;
                let ny = y + dy - 1;
                if nx < self.width && ny < self.height {
                    // 1 in 3 chance of spawning a cell
                    if rand::rand() < (u32::MAX / 3) {
                        self.set_cell(nx, ny, Cell::alive());
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

struct CellColour {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Default for CellColour {
    fn default() -> Self {
        Self {
            r: 8.,
            g: 145.,
            b: 178.,
            a: 255.,
        }
    }
}

struct Spawn {
    interact_size: f32,
    timer_size: f32,
    timer: f32,
    spawn: bool,
}

impl Default for Spawn {
    fn default() -> Self {
        Self {
            interact_size: 1.,
            timer_size: 5.,
            timer: 1.,
            spawn: true,
        }
    }
}

struct RuleSet {
    include_self: bool,
    state_machine: String,
    defined: usize,
}

static GAME_OF_LIFE_STATE_MACHINE: &'static str = r#"{
    "0": {
        "3": 1
    },
    "1": {
        "0": 0,
        "1": 0,
        "4": 0,
        "5": 0,
        "6": 0,
        "7": 0,
        "8": 0,
        "9": 0
    }
}"#;

static HIGHLIFE_STATE_MACHINE: &'static str = r#"{
    "0": {
        "3": 1,
        "6": 1
    },
    "1": {
        "0": 0,
        "1": 0,
        "4": 0,
        "5": 0,
        "6": 0,
        "7": 0,
        "8": 0,
        "9": 0
    }
}"#;

impl Default for RuleSet {
    fn default() -> Self {
        Self {
            include_self: false,
            state_machine: String::from(GAME_OF_LIFE_STATE_MACHINE),
            defined: 0,
        }
    }
}

type RuleSetStateMachine = HashMap<u8, HashMap<u8, u8>>;

struct Config {
    spawn: Spawn,
    cell_colour: CellColour,
    rule_set: RuleSet,
    paused: bool,
    step_time: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            spawn: Default::default(),
            cell_colour: Default::default(),
            rule_set: Default::default(),
            paused: false,
            step_time: 1. / 6.,
        }
    }
}

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

    let mut show_config: bool = false;

    // Default to cyan-600
    let mut config = Config::default();
    let mut defined_rule_ui: usize = 0;

    let colour_slider_range = 0f32..255f32;

    loop {
        elapsed_frame += get_frame_time();
        if elapsed_frame > config.step_time && !config.paused {
            elapsed_frame = 0.0;

            // Only update the world if the game is an 'update frame'
            let state_machine = serde_json::from_str(&config.rule_set.state_machine);

            match state_machine {
                Ok(sm) => world = world.next_generation(sm),
                Err(e) => println!("Ruleset error: {e}"),
            }
        }

        // Interactivity: click to add cells in a 5x5 square around the click
        if is_mouse_button_down(MouseButton::Left) {
            let x = (mouse_position().0 / GRID_SIZE as f32) as isize;
            let y = (mouse_position().1 / GRID_SIZE as f32) as isize;

            world.spawn_group(x, y, config.spawn.interact_size as isize);
        }

        // Spawn some random cells
        elapsed_spawn += get_frame_time();
        if config.spawn.spawn && elapsed_spawn > config.spawn.timer && !config.paused {
            elapsed_spawn = 0.0;
            let x = rand::rand() as isize % width;
            let y = rand::rand() as isize % height;
            world.spawn_group(x, y, config.spawn.timer_size as isize);
        }

        // Clear the frame
        clear_background(WORLD_COLOUR);

        // Render the world
        let cell_colour = color_u8!(
            config.cell_colour.r,
            config.cell_colour.g,
            config.cell_colour.b,
            config.cell_colour.a
        );

        for y in 0..world.height {
            for x in 0..world.width {
                let cell = world.get_cell(x, y);
                if cell.state == 1 {
                    draw_rectangle(
                        x as f32 * GRID_SIZE as f32 + OFFSET as f32,
                        y as f32 * GRID_SIZE as f32 + OFFSET as f32,
                        CELL_SIZE as f32,
                        CELL_SIZE as f32,
                        cell_colour,
                    );
                }
            }
        }

        // Draw config ui

        if show_config {
            if !Window::new(
                hash!(),
                Vec2::new(0.0, 0.0),
                Vec2::new(screen_width() / 2.0, screen_height() / 2.0),
            )
            .movable(false)
            .label("Config")
            .close_button(true)
            .ui(&mut root_ui(), |ui| {
                ui.tree_node(hash!(), "Spawn", |tree_ui| {
                    let spawn_size_range = 0f32..5f32;
                    tree_ui.slider(
                        hash!(),
                        "Interact Size",
                        spawn_size_range.clone(),
                        &mut config.spawn.interact_size,
                    );
                    tree_ui.slider(
                        hash!(),
                        "Periodic Spawn Size",
                        spawn_size_range.clone(),
                        &mut config.spawn.timer_size,
                    );
                    tree_ui.slider(hash!(), "Spawn time", 0f32..10f32, &mut config.spawn.timer);
                    tree_ui.checkbox(hash!(), "Periodic Spawns", &mut config.spawn.spawn);
                });

                config.spawn.interact_size = (config.spawn.interact_size as isize) as f32;
                config.spawn.timer_size = (config.spawn.timer_size as isize) as f32;

                ui.separator();

                ui.tree_node(hash!(), "Cell Colour", |tree_ui| {
                    tree_ui.separator();
                    tree_ui.slider(
                        hash!(),
                        "Red",
                        colour_slider_range.clone(),
                        &mut config.cell_colour.r,
                    );
                    tree_ui.slider(
                        hash!(),
                        "Green",
                        colour_slider_range.clone(),
                        &mut config.cell_colour.g,
                    );
                    tree_ui.slider(
                        hash!(),
                        "Blue",
                        colour_slider_range.clone(),
                        &mut config.cell_colour.b,
                    );
                    tree_ui.slider(
                        hash!(),
                        "Aplha",
                        colour_slider_range.clone(),
                        &mut config.cell_colour.a,
                    );
                });

                ui.separator();

                ui.tree_node(hash!(), "Rule Set", |tree_ui| {
                    tree_ui.checkbox(hash!(), "Include Self", &mut config.rule_set.include_self);

                    tree_ui.label(None, "State Machine");
                    tree_ui.editbox(
                        hash!(),
                        Vec2::new(screen_width() / 2.0 - 10.0, screen_width() / 4.0),
                        &mut config.rule_set.state_machine,
                    );

                    tree_ui.combo_box(
                        hash!(),
                        "Defined Rules",
                        &["Game of Life", "Highlife"],
                        &mut defined_rule_ui,
                    );
                    if defined_rule_ui != config.rule_set.defined {
                        config.rule_set.defined = defined_rule_ui;
                        match defined_rule_ui {
                            0 => {
                                config.rule_set.state_machine =
                                    GAME_OF_LIFE_STATE_MACHINE.to_string()
                            }
                            1 => config.rule_set.state_machine = HIGHLIFE_STATE_MACHINE.to_string(),
                            _ => (),
                        }
                    }
                });

                ui.separator();

                ui.checkbox(hash!(), "Pause", &mut config.paused);

                ui.slider(hash!(), "Step Time", 0f32..2f32, &mut config.step_time);

                ui.separator();

                if ui.button(None, "Reset") {
                    world.reset();
                }
            }) {
                show_config = false;
            }
        } else {
            if Button::new("Config")
                .size(Vec2::new(screen_width() / 20.0, screen_width() / 20.0))
                .ui(&mut root_ui())
            {
                show_config = true;
            }
        }

        // Get next frame
        next_frame().await
    }
}
