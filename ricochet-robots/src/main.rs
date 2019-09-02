use std::collections::HashSet;
use std::collections::VecDeque;
use std::env;
use std::hash::{Hash, Hasher};

extern crate xorshift;
use xorshift::{Rng, SeedableRng, Xorshift128};
type BoardRng = Xorshift128;

extern crate rand;
use rand::prelude::SliceRandom;

extern crate time;
use time::precise_time_ns;

use std::cmp;

extern crate atoi;
use atoi::atoi;

extern crate itertools;
use itertools::Itertools;

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub struct Pos {
	y: i8,
	x: i8,
}

#[derive(Debug)]
struct WallPos {
	y: i8,
	x: i8,
	d: i8,
}

const ROBOTS_COUNT: usize = 4;

#[derive(Debug)]
pub struct Board {
	w: usize,
	h: usize,
	walls: Vec<WallPos>,
	walldist: Vec<Vec<[usize; 4]>>,
	robots: [Pos; ROBOTS_COUNT],
}

const DIRECTIONS: [Pos; 4] = [
	Pos { y: 1, x: 0 },
	Pos { y: 0, x: 1 },
	Pos { y: -1, x: 0 },
	Pos { y: 0, x: -1 },
];

impl Board {
	fn good_board(&mut self) -> bool {
		let mut gone = vec![vec![false; self.w]; self.h];

		fn dfs<'a>(gone: &'a mut Vec<Vec<bool>>, self_: &'a Board, y: usize, x: usize) -> usize {
			//println!("{} {}",y,x);
			if gone[y][x] {
				return 0;
			}
			gone[y][x] = true;
			let mut res = 1;
			for i in 0..4 {
				if self_.walldist[y][x][i] <= 0 {
					continue;
				}
				let ty = (y as i8 + DIRECTIONS[i].y) as usize;
				let tx = (x as i8 + DIRECTIONS[i].x) as usize;
				if ty < self_.h && tx < self_.w {
					res += dfs(gone, self_, ty, tx);
				}
			}
			return res;
		}

		let cn = dfs(&mut gone, self, 0, 0);
		//println!("{}",cn);
		if cn != self.h * self.w {
			// all cells aren't connected
			return false;
		}

		//println!("{:?}",self.board);

		for y in 0..self.h {
			for x in 0..self.w {
				let mut d = 0;
				for i in 0..4 {
					if self.walldist[y][x][i] > 0 {
						d += 1;
					}
				}
				//println!("{}",d);
				if d < 2 {
					// This cell is not interesting.
					return false;
				}
			}
		}

		return true;
	}

	fn init(&mut self, mut rng: BoardRng, wall_num: usize) {
		self.walldist = vec![vec![[0; 4]; self.w]; self.h];

		//println!("{} {} {} {}",self.board.len(), self.h, self.board[0].len(), self.w);
		println!("{}", rng.gen_range(0, 1000));
		for y in 0..self.h {
			for x in 0..self.w {
				self.walldist[y][x] = [self.h - 1 - y, self.w - 1 - x, y, x];
			}
		}

		for _ in 0..wall_num {
			let mem_walldist = self.walldist.clone();
			let mut add_walls = vec![];
			let cy = rng.gen_range(0, self.h);
			let cx = rng.gen_range(0, self.w);
			{
				let y = cy + rng.gen_range(0, 2);
				let x = cx;
				if 0 < y && y < self.h {
					add_walls.push(WallPos {
						y: y as i8,
						x: x as i8,
						d: 0,
					});

					for ty in 0..y {
						self.walldist[ty][x][0] = cmp::min(y - 1 - ty, self.walldist[ty][x][0]);
					}
					for ty in y..self.h {
						self.walldist[ty][x][2] = cmp::min(ty - y, self.walldist[ty][x][2]);
					}
				}
			}
			{
				let y = cy;
				let x = cx + rng.gen_range(0, 2);
				if 0 < x && x < self.w {
					add_walls.push(WallPos {
						y: y as i8,
						x: x as i8,
						d: 1,
					});
					for tx in 0..x {
						self.walldist[y][tx][1] = cmp::min(x - 1 - tx, self.walldist[y][tx][1]);
					}
					for tx in x..self.w {
						self.walldist[y][tx][3] = cmp::min(tx - x, self.walldist[y][tx][3]);
					}
				}
			}

			if self.good_board() {
				println!("add walls {:?}", add_walls);
				self.walls.append(&mut add_walls);
			} else {
				self.walldist = mem_walldist;
			}
		}

		let mut i = 0;
		while i < 4 {
			let tp = Pos {
				y: rng.gen_range(0, self.h) as i8,
				x: rng.gen_range(0, self.w) as i8,
			};
			let mut ok = true;
			for j in 0..i {
				ok &= tp != self.robots[j];
			}
			if ok {
				self.robots[i] = tp;
				i += 1;
			}
		}
	}
	pub fn new(board_h: usize, board_w: usize, rng: BoardRng, wall_num: usize) -> Board {
		let mut res = Board {
			w: board_w,
			h: board_h,
			walls: vec![],
			walldist: vec![],
			robots: [Pos { y: 0, x: 0 }; ROBOTS_COUNT],
		};
		res.init(rng, wall_num);
		return res;
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Move {
	c: usize,
	d: usize,
}

struct State<'a> {
	bo: &'a Board,
	robots: [Pos; ROBOTS_COUNT],
	//log: SinglyLinkedList
	log: usize,
}

impl<'a> State<'a> {
	pub fn init_state(bo: &'a Board) -> State<'a> {
		//State{bo: &bo,robots: bo.robots.clone(), log: SinglyLinkedList::nil()}
		State {
			bo: &bo,
			robots: bo.robots.clone(),
			log: 1,
		}
	}

	fn move_to(&self, robot_index: usize, robot_dir: usize) -> Option<State<'a>> {
		let dir = &DIRECTIONS[robot_dir];
		let mut p = self.robots[robot_index];
		let mut mind = self.bo.walldist[p.y as usize][p.x as usize][robot_dir] as i8;
		//removing "as i8" by changing type of walldist doesn't make well difference.

		// if mind == 0 { return None } //pruning with little (0.2~3sec) speedup.
		/*
		if robot_dir == 2 {
			for j in 0..4 {
				if j != robot_index {
					if self.robots[j].x == p.x && self.robots[j].y < p.y {
						mind = cmp::min(mind,p.y - self.robots[j].y - 1);
					}
				}
			}
		} else if robot_dir == 0 {
			for j in 0..4 {
				if j != robot_index {
					if self.robots[j].x == p.x && self.robots[j].y > p.y {
						mind = cmp::min(mind,self.robots[j].y - p.y - 1);
					}
				}
			}
		} else {
			for j in 0..4 {
				if j != robot_index {
					let dx = self.robots[j].x - p.x;
					if dx.signum() == dir.x.signum() && self.robots[j].y == p.y {
						mind = cmp::min(mind,dx.abs()-1);
					}
				}
			}
		}
		//unloling also has little speedup (0.2~0.3 sec)
		*/
		for j in 0..4 {
			if j != robot_index {
				let dx = self.robots[j].x - p.x;
				let dy = self.robots[j].y - p.y;
				if dx.signum() == dir.x.signum() && dy.signum() == dir.y.signum() {
					if dx.signum() == 0 {
						mind = cmp::min(mind, dy.abs() - 1);
					} else {
						mind = cmp::min(mind, dx.abs() - 1);
					}
				}
			}
		}

		if mind == 0 {
			return None;
		}

		p = Pos {
			y: p.y + dir.y * mind,
			x: p.x + dir.x * mind,
		};

		let tolog = self.log << 4 | robot_index << 2 | robot_dir; //self.log.cons(Move{c: robot_index,d: robot_dir});
		let mut res = State {
			bo: self.bo,
			robots: self.robots.clone(),
			log: tolog,
		};
		res.robots[robot_index] = p;
		Some(res)
	}

	fn enumerate_states(&self) -> Vec<State<'a>> {
		let mut res = Vec::with_capacity(16);
		for i in 0..self.robots.len() {
			for j in 0..4 {
				if let Some(ts) = self.move_to(i, j) {
					res.push(ts);
				}
			}
		}
		return res;
	}
}

impl<'a> PartialEq for State<'a> {
	fn eq(&self, ts: &State) -> bool {
		return self.robots == ts.robots;
	}
}
impl<'a> Eq for State<'a> {}

impl<'a> Hash for State<'a> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		//Surprisingly, this makes program very slowly!
		//:thinking_face:
		// self.robots[0].y.hash(state);
		self.robots.hash(state);
	}
}

pub fn bfs<'a, 'b>(target: u8, bo: &'a Board) -> ((usize, Pos), Vec<Move>) {
	let init = State::init_state(&bo);
	//let mut res = init.log.head.clone();
	let mut res = init.log;
	let mut goal = (0, init.robots[0]);

	let mut gone: HashSet<State> = HashSet::new();
	//let mut gone: HashSet<Vec<Pos>> = HashSet::new();

	let mut que = VecDeque::new();
	let mut depth = 0;
	que.push_back(Some(init));
	que.push_back(None);

	let mut found = vec![vec![[false; ROBOTS_COUNT]; bo.w]; bo.h];
	let mut found_count = 0;
	let max_pattern_num = bo.h * bo.w * bo.robots.len();

	let mut dnum = 1;
	while let Some(st) = que.pop_front() {
		match st {
			Some(st) => {
				if !gone.contains(&st) {
					dnum += 1;
					//println!("{:?}",st.robots);
					let mut ok = false;
					for i in 0..st.robots.len() {
						let p = st.robots[i];
						if !found[p.y as usize][p.x as usize][i] {
							//println!("{} {} {} : {} ",p.y,p.x,i,depth);
							found[p.y as usize][p.x as usize][i] = true;
							found_count += 1;
							//res = st.log.head.clone();
							res = st.log;
							goal = (i, p);
							if depth >= target || found_count >= max_pattern_num {
								ok = true;
								break;
							}
						}
					}
					if ok {
						break;
					}
					for ts in st.enumerate_states() {
						//moving gone.contains & gone.insert to here decreased speed.
						//I don't understand why this happened. :thinking_face:
						que.push_back(Some(ts));
					}
					gone.insert(st);
				}
			}
			None => {
				depth += 1;
				if depth > target {
					break;
				}
				println!("{} {}", depth, dnum);
				dnum = 0;
				que.push_back(None);
			}
		}
	}

	// Faster!!. Haee! 0.69s to 0.53s
	let mut l = vec![];
	while res > 1 {
		let c = (res & 12) >> 2;
		let d = res & 3;
		l.push(Move { c: c, d: d });
		res >>= 4;
	}

	return (goal, l);
	//let l = SinglyLinkedList{head: res};
	//return (goal,l.to_vec());
}

fn main() {
	let args: Vec<String> = env::args().collect();
	let (depth, board_h, board_w, wall_num) = match args[1..5]
		.into_iter()
		.map(|x| atoi(x.as_bytes()))
		.tuples()
		.next()
	{
		Some((Some(a), Some(b), Some(c), Some(d))) => (a, b, c, d),
		v => panic!(
			"invalid argument. expect \"depth board_h board_w wall_num\", got {:?}.",
			v
		),
	};

	let now = precise_time_ns();
	let states = [now, now];
	let stdrng = SeedableRng::from_seed(&states[..]);
	let mut bo = Board::new(board_h, board_w, stdrng, wall_num);
	let ((mut goalcolour, goalpos), mut log) = bfs(depth as u8, &bo);

	//randomize colour
	let mut rng = rand::thread_rng();
	let mut perm: Vec<usize> = (0..bo.robots.len()).collect();
	perm.shuffle(&mut rng);
	let mut perminv: Vec<usize> = vec![0; perm.len()];
	for i in 0..perm.len() {
		perminv[perm[i]] = i;
	}

	goalcolour = perm[goalcolour];
	log = log
		.into_iter()
		.map(|x| Move {
			c: perm[x.c],
			d: x.d,
		})
		.rev()
		.collect();

	{
		let copy = bo.robots;
		for i in 0..ROBOTS_COUNT {
			bo.robots[i] = copy[perminv[i]];
		}
	}

	println!("{:?}", bo);
	println!("{:?}", goalcolour);
	println!("{:?}", goalpos);
	println!("{:?}", log);
}
