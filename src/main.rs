use std::collections::HashMap;

use macroquad::{
    prelude::*,
    rand::ChooseRandom,
    ui::{hash, root_ui, widgets::Window, Skin},
};

use crate::ruleset::{Ruleset, RulesetColour};

mod rule_parsing;
mod ruleset;

#[derive(Clone, Debug, PartialEq)]
struct Cell {
    state: String,
    colour: Color,
}

impl From<RulesetColour> for Color {
    fn from(value: RulesetColour) -> Self {
        match value {
            RulesetColour::Rgba(r, g, b, a) => color_u8!(r, g, b, a),
            RulesetColour::Hex(s) => {
                if s.len() == 9 && s.starts_with('#') {
                    let r = u8::from_str_radix(&s[1..3], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&s[3..5], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&s[5..7], 16).unwrap_or(0);
                    let a = u8::from_str_radix(&s[7..9], 16).unwrap_or(0);
                    color_u8!(r, g, b, a)
                } else {
                    color_u8!(0, 0, 0, 0)
                }
            }
        }
    }
}

struct World {
    width: isize,
    height: isize,
    cells: Vec<Cell>,
    ruleset: Ruleset,
}

impl World {
    fn new(width: isize, height: isize, ruleset: Ruleset) -> Option<Self> {
        let default_state = ruleset.default_state.clone();
        if let Some(state) = ruleset.states.get(&default_state) {
            Some(World {
                width,
                height,
                cells: vec![
                    Cell {
                        state: default_state,
                        colour: state.colour.clone().into()
                    };
                    (width * height) as usize
                ],
                ruleset,
            })
        } else {
            println!("No States defined");
            None
        }
    }

    fn reset(&mut self) {
        let default_state = self.ruleset.default_state.clone();
        if let Some(state) = self.ruleset.states.get(&default_state) {
            self.cells = vec![
                Cell {
                    state: default_state,
                    colour: state.colour.clone().into()
                };
                (self.width * self.height) as usize
            ]
        }
    }

    fn randomise(&mut self) {
        let states: Vec<String> = self.ruleset.states.keys().cloned().collect();
        for cell in &mut self.cells {
            let name = states.choose().unwrap();
            let state = self.ruleset.states.get(name).expect("Unreachable");
            cell.state = name.clone();
            cell.colour = state.colour.clone().into();
        }
    }

    fn get_index(&self, x: isize, y: isize) -> usize {
        let x = if x < 0 { self.width + x } else { x };
        let x = if x >= self.width { x - self.width } else { x };
        let y = if y < 0 { self.height + y } else { y };
        let y = if y >= self.height { y - self.height } else { y };
        (y * self.width + x) as usize
    }

    fn get_cell(&self, x: isize, y: isize) -> &Cell {
        &self.cells[self.get_index(x, y)]
    }

    fn set_cell(&mut self, x: isize, y: isize, cell: Cell) {
        let index = self.get_index(x, y);
        self.cells[index] = cell;
    }

    fn get_neighbourhood(&self, x: isize, y: isize) -> HashMap<String, usize> {
        let mut neighbour_counts = HashMap::new();

        for (dx, dy) in self.ruleset.neighbourhood.get_neighbours() {
            let (a, b) = (x + dx, y + dy);
            let cell = self.get_cell(a, b);

            // TODO: get rid of this clone
            neighbour_counts
                .entry(cell.state.clone())
                .and_modify(|v| *v += 1)
                .or_insert(1);
        }

        neighbour_counts
    }

    fn next_generation(&mut self) {
        let mut new_generation = self.cells.clone();

        for y in 0..self.height {
            for x in 0..self.width {
                let neighbour_counts = self.get_neighbourhood(x, y);
                let current_cell = self.get_cell(x, y);

                // TODO: Remove this clone
                if let Some(rules) = self.ruleset.states.get(&current_cell.state) {
                    if let Some(next) = rules.transition(&neighbour_counts) {
                        let colour = next
                            .paint
                            .as_ref()
                            .unwrap_or(&self.ruleset.states.get(&next.next).unwrap().colour);
                        new_generation[self.get_index(x, y)] = Cell {
                            state: next.next.clone(),
                            colour: colour.clone().into(),
                        };
                        // TODO: can I remove these clones?
                    }
                } else {
                    println!(
                        "No state rules found in {:?} with ruleset {:?}",
                        current_cell, self.ruleset
                    );
                }
            }
        }

        self.cells = new_generation;
    }

    fn spawn_group(&mut self, x: isize, y: isize, size: isize, state: &str) {
        let state_name = state.to_string();
        let state_definition = self.ruleset.states.get(state);
        if state_definition.is_none() {
            println!("No {} defined", state);
            return;
        }

        let state_colour = state_definition.unwrap().colour.clone();

        if size == 0 {
            return;
        } else if size == 1 {
            self.set_cell(
                x,
                y,
                Cell {
                    state: state_name,
                    colour: state_colour.into(),
                },
            );
            return;
        }

        for dx in 0..size {
            for dy in 0..size {
                let nx = x + dx - 1;
                let ny = y + dy - 1;
                if nx < self.width && ny < self.height {
                    // 1 in 3 chance of spawning a cell
                    if rand::rand() < (u32::MAX / 3) {
                        self.set_cell(
                            nx,
                            ny,
                            Cell {
                                state: state_name.clone(),
                                colour: state_colour.clone().into(),
                            },
                        );
                    }
                }
            }
        }
    }
}

static WORLD_COLOUR: Color = color_u8!(0, 0, 0, 0);
static GRID_SIZE: usize = 15;
static CELL_SIZE: usize = 14;
static OFFSET: usize = (GRID_SIZE - CELL_SIZE) / 2;

struct Spawn {
    interact_size: f32,
    timer_size: f32,
    timer: f32,
    spawn: bool,
    spawn_state: usize,
}

impl Default for Spawn {
    fn default() -> Self {
        Self {
            interact_size: 1.,
            timer_size: 5.,
            timer: 1.,
            spawn: false,
            spawn_state: 0,
        }
    }
}

static GAME_OF_LIFE_STATE_MACHINE: &str = include_str!("../rulesets/game_of_life.json");
static HIGHLIFE_STATE_MACHINE: &str = include_str!("../rulesets/highlife.json");
static WIREWORLD_STATE_MACHINE: &str = include_str!("../rulesets/wireworld.json");
static IMMIGRATION_STATE_MACHINE: &str = include_str!("../rulesets/immigration.json");
static CYCLIC_STATE_MACHINE: &str = include_str!("../rulesets/cyclic.json");

struct Config {
    spawn: Spawn,
    ruleset: String,
    paused: bool,
    step_time: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            spawn: Default::default(),
            ruleset: GAME_OF_LIFE_STATE_MACHINE.to_string(),
            paused: false,
            step_time: 0.5,
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

    let mut elapsed_frame: f32 = 0.0;
    let mut elapsed_spawn: f32 = 0.0;

    let mut show_config: bool = false;

    let mut config = Config::default();

    let mut defined_rule_ui: usize = 0;
    let mut previous_defined_rule_ui: usize = 0;

    let ruleset: Ruleset = serde_json::from_str(&config.ruleset).unwrap();
    println!("\n\n {:?} \n\n", ruleset);

    let mut states: Vec<String> = ruleset.states.keys().cloned().collect();
    // combo boxes only take &[&str], precreate to avoid allocating this every frame
    let mut states_ref: Vec<&str> = states.iter().map(|s| s.as_str()).collect();

    let mut world = World::new(width, height, ruleset).unwrap();
    world.randomise();

    let mut ruleset_changed = false;

    // UI Skins
    let white_text_style = root_ui()
        .style_builder()
        .text_color(color_u8!(255, 255, 255, 255))
        .build();
    let white_text_skin = Skin {
        label_style: white_text_style,
        ..root_ui().default_skin()
    };

    loop {
        // Only update the world if the game is an 'update frame'
        elapsed_frame += get_frame_time();
        if elapsed_frame > config.step_time && !config.paused {
            elapsed_frame = 0.0;

            // reset the world if the ruleset has changed to a valid config
            if ruleset_changed {
                ruleset_changed = false;
                let ruleset: Result<Ruleset, serde_json::Error> =
                    serde_json::from_str(&config.ruleset);

                match ruleset {
                    Ok(ok_ruleset) => {
                        states = ok_ruleset.states.keys().cloned().collect();
                        // combo boxes only take &[&str], precreate to avoid allocating this every frame
                        states_ref = states.iter().map(|s| s.as_str()).collect();
                        if let Some(new_world) = World::new(width, height, ok_ruleset) {
                            world = new_world;
                        } else {
                            println!("Error creating new world from ruleset")
                        }
                    }
                    Err(e) => println!("Ruleset error: {e}"),
                }
            }

            world.next_generation();
        }

        // Interactivity: click to add cells in a 5x5 square around the click
        if !show_config && is_mouse_button_down(MouseButton::Left) {
            let x = (mouse_position().0 / GRID_SIZE as f32) as isize;
            let y = (mouse_position().1 / GRID_SIZE as f32) as isize;

            world.spawn_group(
                x,
                y,
                config.spawn.interact_size as isize,
                &states[config.spawn.spawn_state],
            );
        }

        // Spawn some random cells
        elapsed_spawn += get_frame_time();
        if config.spawn.spawn && elapsed_spawn > config.spawn.timer && !config.paused {
            elapsed_spawn = 0.0;
            let x = rand::rand() as isize % width;
            let y = rand::rand() as isize % height;
            world.spawn_group(
                x,
                y,
                config.spawn.timer_size as isize,
                &states[config.spawn.spawn_state],
            );
        }

        // Clear the frame

        clear_background(WORLD_COLOUR);

        // Render the world

        for y in 0..world.height {
            for x in 0..world.width {
                let cell = world.get_cell(x, y);
                draw_rectangle(
                    x as f32 * GRID_SIZE as f32 + OFFSET as f32,
                    y as f32 * GRID_SIZE as f32 + OFFSET as f32,
                    CELL_SIZE as f32,
                    CELL_SIZE as f32,
                    cell.colour,
                );
            }
        }

        if is_key_pressed(KeyCode::Q) {
            show_config = !show_config;
        }

        if is_key_pressed(KeyCode::E) {
            config.spawn.spawn_state = (config.spawn.spawn_state + 1) % states.len();
        }

        if is_key_pressed(KeyCode::W) {
            config.spawn.spawn_state = (states.len() + config.spawn.spawn_state - 1) % states.len();
        }

        if is_key_pressed(KeyCode::Space) {
            config.paused = !config.paused;
        }

        // Draw config ui

        if show_config
            && !Window::new(
                hash!(),
                Vec2::new(screen_width() * 0.1, screen_height() * 0.1),
                Vec2::new(screen_width() * 0.8, screen_height() * 0.8),
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
                    tree_ui.combo_box(
                        hash!(),
                        "Spawn state",
                        &states_ref,
                        &mut config.spawn.spawn_state,
                    );
                });

                config.spawn.interact_size = (config.spawn.interact_size as isize) as f32;
                config.spawn.timer_size = (config.spawn.timer_size as isize) as f32;

                ui.separator();

                ui.tree_node(hash!(), "Rule Set", |tree_ui| {
                    tree_ui.combo_box(
                        hash!(),
                        "Defined Rules",
                        &[
                            "Game of Life",
                            "Highlife",
                            "Immigration",
                            "Cyclic",
                            "Wireworld",
                        ],
                        &mut defined_rule_ui,
                    );

                    if defined_rule_ui != previous_defined_rule_ui {
                        match defined_rule_ui {
                            0 => config.ruleset = GAME_OF_LIFE_STATE_MACHINE.to_string(),
                            1 => config.ruleset = HIGHLIFE_STATE_MACHINE.to_string(),
                            2 => config.ruleset = IMMIGRATION_STATE_MACHINE.to_string(),
                            3 => config.ruleset = CYCLIC_STATE_MACHINE.to_string(),
                            4 => config.ruleset = WIREWORLD_STATE_MACHINE.to_string(),
                            _ => unreachable!(),
                        }
                        ruleset_changed = true;
                        previous_defined_rule_ui = defined_rule_ui;
                    }

                    tree_ui.label(None, "State Machine");
                    if tree_ui.editbox(
                        hash!(),
                        Vec2::new(screen_width() * 0.75, screen_height() * 0.75),
                        &mut config.ruleset,
                    ) {
                        ruleset_changed = true;
                    };
                });

                ui.separator();

                ui.checkbox(hash!(), "Pause", &mut config.paused);

                ui.slider(hash!(), "Step Time", 0f32..2f32, &mut config.step_time);

                ui.separator();

                if ui.button(None, "Reset") {
                    world.reset();
                }

                if ui.button(None, "Randomise") {
                    world.randomise();
                }
            })
        {
            show_config = false;
        }

        root_ui().push_skin(&white_text_skin);

        if config.paused {
            root_ui().label(Vec2::new(0.0, screen_height() - 32.0), "Paused!");
        }
        root_ui().label(
            Vec2::new(0.0, screen_height() - 16.0),
            &format!("Selected: {}", states[config.spawn.spawn_state]),
        );

        root_ui().pop_skin();

        // Get next frame
        next_frame().await
    }
}
