use clearscreen::clear;
use rand::prelude::*;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

type QTable = HashMap<(State, usize), f32>;

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
struct State {
    x: i32,
    y: i32,
}

enum Outcome {
    Ongoing,
    Agent1Goal,
    Agent2Goal,
}

struct Env {
    grid: Vec<Vec<i32>>, // 0 = empty, 1 = wall
    agent: State,
    agent2: State,
    goal: State,
}

const ACTIONS: [(i32, i32); 4] = [
    (0, -1), // up
    (0, 1),  // down
    (-1, 0), // left
    (1, 0),  // right
];

fn main() {
    let mut q1: QTable = HashMap::new();
    let mut q2: QTable = HashMap::new();

    let mut times_won_1 = 0;
    let mut times_won_2 = 0;

    for episode in 0..1000 {
        let mut env = Env {
            grid: vec![
                vec![0, 0, 0, 0, 1, 0, 0, 0, 0],
                vec![1, 1, 0, 1, 1, 1, 0, 1, 1],
                vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
                vec![1, 1, 1, 1, 0, 1, 1, 1, 1],
                vec![0, 1, 0, 1, 0, 0, 0, 1, 0],
                vec![0, 0, 0, 0, 0, 1, 0, 0, 0],
                vec![1, 0, 1, 1, 1, 1, 1, 0, 1],
                vec![1, 0, 0, 0, 0, 0, 0, 0, 1],
            ],
            agent: State { x: 0, y: 0 },
            agent2: State { x: 8, y: 0 },
            goal: State { x: 4, y: 7 },
        };

        for step in 0..100 {
            let state1 = env.agent;
            let state2 = env.agent2;

            let action1 = choose_action(state1, &q1, episode);
            let action2 = choose_action(state2, &q2, episode);

            let (reward1, reward2, outcome) = env.step_both(action1, action2);

            let next_state1 = env.agent;
            let next_state2 = env.agent2;

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
                "Episode: {} Step: {} | A Wins: {} | B Wins: {}",
                episode, step, times_won_1, times_won_2
            );
            print_env(&env);
            thread::sleep(Duration::from_millis(100));
        }
    }
}

fn distance(a: State, b: State) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

impl Env {
    fn move_agent(&self, pos: State, action: usize) -> State {
        let (dx, dy) = ACTIONS[action];
        let new_x = pos.x + dx;
        let new_y = pos.y + dy;

        // bounds check
        if new_x < 0
            || new_y < 0
            || new_y as usize >= self.grid.len()
            || new_x as usize >= self.grid[0].len()
        {
            return pos;
        }

        // wall check
        if self.grid[new_y as usize][new_x as usize] == 1 {
            return pos;
        }

        State { x: new_x, y: new_y }
    }

    fn step_both(&mut self, action1: usize, action2: usize) -> (f32, f32, Outcome) {
        let old1 = self.agent;
        let old2 = self.agent2;

        let new1 = self.move_agent(self.agent, action1);
        let new2 = self.move_agent(self.agent2, action2);

        self.agent = new1;
        self.agent2 = new2;

        // --- GOAL CHECK ---
        if self.agent == self.goal {
            return (10.0, -10.0, Outcome::Agent1Goal);
        }

        if self.agent2 == self.goal {
            return (-10.0, 10.0, Outcome::Agent2Goal);
        }

        // --- DISTANCE REWARD ---
        let r1 = if distance(new1, self.goal) < distance(old1, self.goal) {
            0.0
        } else {
            -0.2
        };

        let r2 = if distance(new2, self.goal) < distance(old2, self.goal) {
            0.0
        } else {
            -0.2
        };

        (r1, r2, Outcome::Ongoing)
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
            if env.agent.x == x as i32 && env.agent.y == y as i32 {
                print!("A ");
            } else if env.agent2.x == x as i32 && env.agent2.y == y as i32 {
                print!("B ");
            } else if env.goal.x == x as i32 && env.goal.y == y as i32 {
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
