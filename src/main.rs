//use clearscreen::clear;
use clearscreen::clear;
use rand::prelude::*;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

type QTable = HashMap<(State, usize), f32>;

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
struct State {
    up_blocked: bool,
    down_blocked: bool,
    left_blocked: bool,
    right_blocked: bool,
    goal_dx: i32,
    goal_dy: i32,
    last_action: i8,
}

enum Outcome {
    Ongoing,
    Agent1Goal,
    Agent2Goal,
}

struct Env {
    grid: Vec<Vec<i32>>,
    agent: (i32, i32),
    agent2: (i32, i32),
    goal: (i32, i32),
    last_action1: i8,
    last_action2: i8,
}

const ACTIONS: [(i32, i32); 4] = [
    (0, -1), // up
    (0, 1),  // down
    (-1, 0), // left
    (1, 0),  // right
];

fn generate_maze(width: usize, height: usize) -> Vec<Vec<i32>> {
    let mut grid = vec![vec![1; width]; height];
    let mut rng = rand::thread_rng();

    fn carve(x: usize, y: usize, grid: &mut Vec<Vec<i32>>, rng: &mut ThreadRng) {
        grid[y][x] = 0;

        let mut directions = vec![(0isize, -2isize), (0, 2), (-2, 0), (2, 0)];

        directions.shuffle(rng);

        for (dx, dy) in directions {
            let nx = x as isize + dx;
            let ny = y as isize + dy;

            if nx > 0
                && ny > 0
                && nx < (grid[0].len() as isize)
                && ny < (grid.len() as isize)
                && grid[ny as usize][nx as usize] == 1
            {
                // remove wall between
                grid[(y as isize + dy / 2) as usize][(x as isize + dx / 2) as usize] = 0;

                carve(nx as usize, ny as usize, grid, rng);
            }
        }
    }

    carve(1, 1, &mut grid, &mut rng);

    grid[0][0] = 0;
    grid[height - 1][width - 1] = 0;

    grid
}

fn main() {
    let mut q1: QTable = HashMap::new();
    let mut q2: QTable = HashMap::new();

    let mut times_won_1 = 0;
    let mut times_won_2 = 0;

    for episode in 0..1000 {
        let mut env = Env {
            //grid: vec![
            //    //Random Maze Generation
            //    vec![0, 0, 0, 0, 1, 0, 0, 0, 0],
            //    vec![1, 1, 0, 1, 1, 1, 0, 1, 1],
            //    vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            //    vec![0, 1, 0, 1, 0, 1, 0, 1, 0],
            //    vec![0, 1, 0, 1, 1, 1, 0, 1, 0],
            //    vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            //    vec![1, 0, 1, 1, 0, 1, 1, 0, 1],
            //    vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            //    vec![0, 1, 1, 0, 0, 0, 1, 1, 0],
            //    vec![0, 1, 0, 1, 1, 1, 0, 1, 0],
            //    vec![0, 0, 0, 1, 0, 1, 0, 0, 0],
            //    vec![1, 1, 0, 0, 0, 0, 0, 1, 1],
            //    vec![0, 1, 0, 1, 0, 1, 0, 1, 0],
            //],
            grid: generate_maze(13, 15),
            agent: (1, 1),
            agent2: (2, 1),
            goal: (11, 13),
            last_action1: -1,
            last_action2: -1,
        };

        for step in 0..200 {
            let state1 = env.get_state(env.agent, env.last_action1);
            let state2 = env.get_state(env.agent2, env.last_action2);

            let action1 = choose_action(state1, &q1, episode);
            let action2 = choose_action(state2, &q2, episode);

            let (reward1, reward2, outcome) = env.step_both(action1, action2);

            let next_state1 = env.get_state(env.agent, env.last_action1);
            let next_state2 = env.get_state(env.agent2, env.last_action2);

            update_q(&mut q1, state1, action1, reward1, next_state1);
            update_q(&mut q2, state2, action2, reward2, next_state2);

            match outcome {
                Outcome::Agent1Goal => {
                    times_won_1 += 1;
                    break;
                }
                Outcome::Agent2Goal => {
                    times_won_2 += 1;
                    break;
                }
                Outcome::Ongoing => {}
            }

            clear().unwrap();
            println!(
                "Episode: {} Step: {} | Abel Wins: {} | Cain Wins: {}", //Print out an IQ for the bots
                episode, step, times_won_1, times_won_2
            );
            print_env(&env);
            thread::sleep(Duration::from_millis(150));
        }
    }
}

fn distance_pos(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

fn is_reverse(a: usize, b: i8) -> bool {
    match (a, b) {
        (0, 1) | (1, 0) | (2, 3) | (3, 2) => true,
        _ => false,
    }
}

impl Env {
    fn move_agent(&self, pos: (i32, i32), action: usize) -> ((i32, i32), bool) {
        let (dx, dy) = ACTIONS[action];
        let new_x = pos.0 + dx;
        let new_y = pos.1 + dy;

        if new_x < 0
            || new_y < 0
            || new_y as usize >= self.grid.len()
            || new_x as usize >= self.grid[0].len()
        {
            return (pos, true);
        }

        if self.grid[new_y as usize][new_x as usize] == 1 {
            return (pos, true);
        }

        ((new_x, new_y), false)
    }

    fn step_both(&mut self, action1: usize, action2: usize) -> (f32, f32, Outcome) {
        let old1 = self.agent;
        let old2 = self.agent2;

        let (new1, hit1) = self.move_agent(self.agent, action1);
        let (new2, hit2) = self.move_agent(self.agent2, action2);

        self.agent = new1;
        self.agent2 = new2;

        // GOAL
        if self.agent == self.goal {
            return (10.0, -10.0, Outcome::Agent1Goal);
        }
        if self.agent2 == self.goal {
            return (-10.0, 10.0, Outcome::Agent2Goal);
        }

        // WALL PENALTY
        let mut r1 = if hit1 { -5.0 } else { 0.0 };
        let mut r2 = if hit2 { -5.0 } else { 0.0 };

        // DISTANCE SHAPING
        if !hit1 {
            if distance_pos(new1, self.goal) < distance_pos(old1, self.goal) {
                r1 += 0.1;
            } else {
                r1 -= 0.1;
            }
        }

        if !hit2 {
            if distance_pos(new2, self.goal) < distance_pos(old2, self.goal) {
                r2 += 0.1;
            } else {
                r2 -= 0.1;
            }
        }

        // REVERSE PENALTY
        if is_reverse(action1, self.last_action1) {
            r1 -= 1.0;
        }
        if is_reverse(action2, self.last_action2) {
            r2 -= 1.0;
        }

        // INACTION PENALTY
        if new1 == old1 {
            r1 -= 1.0;
        }
        if new2 == old2 {
            r2 -= 1.0;
        }

        self.last_action1 = action1 as i8;
        self.last_action2 = action2 as i8;
        (r1, r2, Outcome::Ongoing)
    }

    fn get_state(&self, pos: (i32, i32), last_action: i8) -> State {
        let (x, y) = pos;

        State {
            up_blocked: self.is_blocked(x, y - 1),
            down_blocked: self.is_blocked(x, y + 1),
            left_blocked: self.is_blocked(x - 1, y),
            right_blocked: self.is_blocked(x + 1, y),
            goal_dx: (self.goal.0 - x).signum(),
            goal_dy: (self.goal.1 - y).signum(),
            last_action,
        }
    }

    fn is_blocked(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || y as usize >= self.grid.len() || x as usize >= self.grid[0].len() {
            return true;
        }

        self.grid[y as usize][x as usize] == 1
    }
}

fn choose_action(state: State, q: &QTable, episode: i32) -> usize {
    let mut rng = rand::thread_rng();

    let epsilon = (0.2_f32 * (0.995_f32.powi(episode))).max(0.01);

    if rng.r#gen::<f32>() < epsilon {
        rng.gen_range(0..4)
    } else {
        (0..4)
            .max_by(|&a, &b| {
                let qa = *q.get(&(state, a)).unwrap_or(&0.0);
                let qb = *q.get(&(state, b)).unwrap_or(&0.0);
                qa.partial_cmp(&qb).unwrap()
            })
            .unwrap()
    }
}

fn update_q(q: &mut QTable, state: State, action: usize, reward: f32, next_state: State) {
    let alpha = 0.1;
    let gamma = 0.9;

    let current = *q.get(&(state, action)).unwrap_or(&0.0);

    let next_max = (0..4)
        .map(|a| *q.get(&(next_state, a)).unwrap_or(&0.0))
        .fold(f32::MIN, f32::max);

    let new_q = current + alpha * (reward + gamma * next_max - current);

    q.insert((state, action), new_q);
}

fn print_env(env: &Env) {
    for y in 0..env.grid.len() {
        for x in 0..env.grid[0].len() {
            if env.agent.0 == x as i32 && env.agent.1 == y as i32 {
                print!("A ");
            } else if env.agent2.0 == x as i32 && env.agent2.1 == y as i32 {
                print!("C ");
            } else if env.goal.0 == x as i32 && env.goal.1 == y as i32 {
                print!("G ");
            } else if env.grid[y][x] == 1 {
                print!("# ");
            } else {
                print!(". ");
            }
        }
        println!();
    }
    println!();
}
