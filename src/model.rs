//! Deterministic, platform-independent Sugarscape. No rendering or wall clock here.
use std::collections::VecDeque;
mod social;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chapter {
    Life,
    Society,
}

impl Chapter {
    pub const ALL: [Self; 2] = [Self::Life, Self::Society];
    pub fn title(self) -> &'static str {
        match self {
            Self::Life => "Life and Death on the Sugarscape",
            Self::Society => "Sex, Culture and Conflict",
        }
    }
    pub fn experiments(self) -> &'static [Experiment] {
        match self {
            Self::Life => &[
                Experiment::Immediate,
                Experiment::Growback,
                Experiment::Replacement,
            ],
            Self::Society => &[Experiment::Sex, Experiment::Culture, Experiment::Conflict],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Experiment {
    Immediate,
    Growback,
    Replacement,
    Sex,
    Culture,
    Conflict,
}

impl Experiment {
    pub const ALL: [Self; 6] = [
        Self::Immediate,
        Self::Growback,
        Self::Replacement,
        Self::Sex,
        Self::Culture,
        Self::Conflict,
    ];
    pub fn chapter(self) -> Chapter {
        match self {
            Self::Immediate | Self::Growback | Self::Replacement => Chapter::Life,
            _ => Chapter::Society,
        }
    }
    pub fn social(self) -> bool {
        self.chapter() == Chapter::Society
    }
    pub fn culture(self) -> bool {
        matches!(self, Self::Culture | Self::Conflict)
    }
    pub fn aging(self) -> bool {
        self == Self::Replacement || self.social()
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::Immediate => "01  Мгновенное восстановление",
            Self::Growback => "02  Ограниченный ресурс",
            Self::Replacement => "03  Жизнь и смерть",
            Self::Sex => "01  Половое размножение",
            Self::Culture => "02  Передача культуры",
            Self::Conflict => "03  Межгрупповой конфликт",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Immediate => {
                "G∞ + M · Сахар полностью восстанавливается перед каждым шагом. Агенты ищут лучшие участки и расходуют запасы."
            }
            Self::Growback => {
                "Gα + M · Сахар восстанавливается постепенно. Конкуренция за ресурс определяет численность выживших."
            }
            Self::Replacement => {
                "Gα + M + R · Гибель от голода или старости. Умершего заменяет новый случайный агент без наследования."
            }
            Self::Sex => {
                "Gα + M + S · Соседи разного пола создают потомство. Население меняется через рождения, голод и старение."
            }
            Self::Culture => {
                "Gα + M + S + K · К размножению добавляется передача культурных признаков соседям. Группы могут меняться."
            }
            Self::Conflict => {
                "Gα + C + S + K · Агенты могут нападать на более бедных членов другой группы, учитывая риск возмездия."
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub size: usize,
    pub population: usize,
    pub vision: u32,
    pub metabolism: u32,
    pub initial_sugar: u32,
    pub capacity: u32,
    pub growback: u32,
    pub min_age: u32,
    pub max_age: u32,
    pub seed: u64,
    pub wrap: bool,
    pub experiment: Experiment,
    pub fertility_start: u32,
    pub female_fertility_end: u32,
    pub male_fertility_end: u32,
    pub female_percent: u32,
    pub culture_tags: u32,
    pub culture_contacts: u32,
    pub combat_loot: u32,
    pub segregated: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size: 50,
            population: 400,
            vision: 6,
            metabolism: 4,
            initial_sugar: 25,
            capacity: 4,
            growback: 1,
            min_age: 60,
            max_age: 100,
            seed: 42,
            wrap: true,
            experiment: Experiment::Growback,
            fertility_start: 15,
            female_fertility_end: 40,
            male_fertility_end: 50,
            female_percent: 50,
            culture_tags: 11,
            culture_contacts: 1,
            combat_loot: 20,
            segregated: false,
        }
    }
}

impl Config {
    pub fn normalized(mut self) -> Self {
        self.size = self.size.clamp(10, 100);
        self.population = self.population.clamp(1, self.size * self.size);
        self.vision = self.vision.clamp(1, 20);
        self.metabolism = self.metabolism.clamp(1, 12);
        self.initial_sugar = self.initial_sugar.clamp(1, 100);
        self.capacity = self.capacity.clamp(1, 12);
        self.growback = self.growback.min(12);
        self.min_age = self.min_age.clamp(1, 500);
        self.max_age = self.max_age.clamp(self.min_age, 500);
        self.fertility_start = self.fertility_start.clamp(1, 100);
        self.female_fertility_end = self.female_fertility_end.clamp(self.fertility_start, 200);
        self.male_fertility_end = self.male_fertility_end.clamp(self.fertility_start, 200);
        self.female_percent = self.female_percent.min(100);
        self.culture_tags = self.culture_tags.clamp(1, 31) | 1;
        self.culture_contacts = self.culture_contacts.min(self.culture_tags);
        self.combat_loot = self.combat_loot.min(100);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    pub sugar: u32,
    pub capacity: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Agent {
    pub id: u64,
    pub pos: usize,
    pub sugar: u64,
    pub vision: u32,
    pub metabolism: u32,
    pub age: u32,
    pub max_age: u32,
    pub female: bool,
    pub tags: u32,
    pub endowment: u64,
    pub parents: Option<[u64; 2]>,
    pub generation: u32,
}

impl Agent {
    pub fn red(&self, tag_count: u32) -> bool {
        self.tags.count_ones() > tag_count / 2
    }
    pub fn fertile(&self, config: &Config) -> bool {
        let end = if self.female {
            config.female_fertility_end
        } else {
            config.male_fertility_end
        };
        self.age >= config.fertility_start
            && self.age <= end
            && self.sugar >= self.endowment
            && self.sugar > self.endowment.div_ceil(2)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stats {
    pub tick: u64,
    pub population: usize,
    pub mean_wealth: f64,
    pub mean_vision: f64,
    pub mean_metabolism: f64,
    pub gini: f64,
    pub sugar: u64,
    pub red_population: usize,
    pub fertile_population: usize,
    pub births: u64,
    pub combat_deaths: u64,
}

/// SplitMix64: identical streams on native and wasm32. Rejection avoids modulo bias.
struct Random(u64);
impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        let n = n as u64;
        let threshold = n.wrapping_neg() % n;
        loop {
            let x = self.next();
            if x >= threshold {
                return (x % n) as usize;
            }
        }
    }
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below((hi - lo + 1) as usize) as u32
    }
    fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            let j = self.below(i + 1);
            values.swap(i, j);
        }
    }
}

pub struct World {
    pub config: Config,
    pub cells: Vec<Cell>,
    pub agents: Vec<Agent>,
    pub history: VecDeque<Stats>,
    pub tick: u64,
    pub starvation_deaths: u64,
    pub age_deaths: u64,
    pub births: u64,
    pub combat_deaths: u64,
    random: Random,
    next_id: u64,
}

impl World {
    pub fn new(config: Config) -> Self {
        let config = config.normalized();
        let n = config.size;
        // Two radial hills, deliberately procedural rather than the book's original data.
        let cells = (0..n * n)
            .map(|i| {
                let x = i % n;
                let y = i / n;
                let distance = |cx: f64, cy: f64| {
                    ((x as f64 / n as f64 - cx).powi(2) + (y as f64 / n as f64 - cy).powi(2)).sqrt()
                };
                let height =
                    (1.0 - distance(0.27, 0.27).min(distance(0.73, 0.73)) / 0.48).clamp(0.0, 1.0);
                let capacity = (height * config.capacity as f64).ceil() as u32;
                Cell {
                    sugar: capacity,
                    capacity,
                }
            })
            .collect();
        let mut world = Self {
            random: Random(config.seed),
            config,
            cells,
            agents: Vec::new(),
            history: VecDeque::new(),
            tick: 0,
            starvation_deaths: 0,
            age_deaths: 0,
            births: 0,
            combat_deaths: 0,
            next_id: 0,
        };
        let mut places: Vec<_> = (0..n * n).collect();
        world.random.shuffle(&mut places);
        for &pos in places.iter().take(world.config.population) {
            let agent = world.new_agent(pos);
            world.agents.push(agent);
        }
        world.record();
        world
    }

    fn new_agent(&mut self, pos: usize) -> Agent {
        let id = self.next_id;
        self.next_id += 1;
        let mut agent = Agent {
            id,
            pos,
            sugar: self
                .random
                .range(5.min(self.config.initial_sugar), self.config.initial_sugar)
                as u64,
            vision: self.random.range(1, self.config.vision),
            metabolism: self.random.range(1, self.config.metabolism),
            age: 0,
            max_age: self.random.range(self.config.min_age, self.config.max_age),
            female: false,
            tags: 0,
            endowment: 0,
            parents: None,
            generation: 0,
        };
        // Keep the first chapter's random stream unchanged.
        if self.config.experiment.social() {
            agent.sugar = agent.sugar.max(2);
            agent.endowment = agent.sugar;
            agent.female = self.random.below(100) < self.config.female_percent as usize;
            let mask = (1_u32 << self.config.culture_tags) - 1;
            agent.tags = if self.config.segregated {
                if pos % self.config.size < self.config.size / 2 {
                    0
                } else {
                    mask
                }
            } else {
                self.random.next() as u32 & mask
            };
        }
        agent
    }

    /// Current cell plus four axial rays; agents do not block sight.
    pub fn visible(&self, pos: usize, vision: u32) -> Vec<(usize, u32)> {
        let n = self.config.size as i32;
        let (x, y) = ((pos % n as usize) as i32, (pos / n as usize) as i32);
        let mut result: Vec<(usize, u32)> = vec![(pos, 0)];
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            for d in 1..=vision.min((n - 1) as u32) {
                let (mut xx, mut yy) = (x + dx * d as i32, y + dy * d as i32);
                if self.config.wrap {
                    xx = xx.rem_euclid(n);
                    yy = yy.rem_euclid(n);
                } else if xx < 0 || yy < 0 || xx >= n || yy >= n {
                    break;
                }
                let p = (yy * n + xx) as usize;
                if let Some((_, distance)) = result.iter_mut().find(|(i, _)| *i == p) {
                    *distance = (*distance).min(d);
                } else {
                    result.push((p, d));
                }
            }
        }
        result
    }

    pub fn step(&mut self) {
        if self.config.experiment.social() {
            self.step_social();
            self.tick += 1;
            self.record();
            return;
        }
        for c in &mut self.cells {
            c.sugar = if self.config.experiment == Experiment::Immediate {
                c.capacity
            } else {
                c.capacity.min(c.sugar + self.config.growback)
            };
        }
        let mut occupied = vec![false; self.cells.len()];
        for a in &self.agents {
            occupied[a.pos] = true;
        }
        // Sequential moves in a newly shuffled order every tick.
        let mut agents = std::mem::take(&mut self.agents);
        self.random.shuffle(&mut agents);
        let mut replacements = 0;
        for mut a in agents {
            occupied[a.pos] = false;
            let mut best = self.cells[a.pos].sugar;
            let mut distance = 0;
            let mut choices = vec![a.pos];
            for (p, d) in self.visible(a.pos, a.vision).into_iter().skip(1) {
                if occupied[p] {
                    continue;
                }
                let sugar = self.cells[p].sugar;
                if sugar > best || (sugar == best && d < distance) {
                    best = sugar;
                    distance = d;
                    choices.clear();
                    choices.push(p);
                } else if sugar == best && d == distance {
                    choices.push(p);
                }
            }
            a.pos = choices[self.random.below(choices.len())];
            a.sugar += self.cells[a.pos].sugar as u64;
            self.cells[a.pos].sugar = 0;
            a.sugar = a.sugar.saturating_sub(a.metabolism as u64);
            a.age += 1;
            let hungry = a.sugar == 0;
            let old = self.config.experiment == Experiment::Replacement && a.age >= a.max_age;
            if hungry || old {
                if hungry {
                    self.starvation_deaths += 1;
                } else {
                    self.age_deaths += 1;
                }
                replacements += 1;
            } else {
                occupied[a.pos] = true;
                self.agents.push(a);
            }
        }
        // Newborns arrive after all moves, so each agent acts at most once per tick.
        if self.config.experiment == Experiment::Replacement && replacements > 0 {
            let mut free: Vec<_> = occupied
                .iter()
                .enumerate()
                .filter_map(|(i, &v)| (!v).then_some(i))
                .collect();
            self.random.shuffle(&mut free);
            for &pos in free.iter().take(replacements) {
                let agent = self.new_agent(pos);
                self.agents.push(agent);
                self.births += 1;
            }
        }
        self.tick += 1;
        self.record();
    }

    pub fn stats(&self) -> Stats {
        let n = self.agents.len() as f64;
        let mean = |sum: f64| if n > 0.0 { sum / n } else { 0.0 };
        let wealth: Vec<_> = self.agents.iter().map(|a| a.sugar as f64).collect();
        Stats {
            tick: self.tick,
            population: self.agents.len(),
            mean_wealth: mean(wealth.iter().sum()),
            mean_vision: mean(self.agents.iter().map(|a| a.vision as f64).sum()),
            mean_metabolism: mean(self.agents.iter().map(|a| a.metabolism as f64).sum()),
            gini: gini(&wealth),
            sugar: self.cells.iter().map(|c| c.sugar as u64).sum(),
            red_population: self
                .agents
                .iter()
                .filter(|a| a.red(self.config.culture_tags))
                .count(),
            fertile_population: self
                .agents
                .iter()
                .filter(|a| a.fertile(&self.config))
                .count(),
            births: self.births,
            combat_deaths: self.combat_deaths,
        }
    }
    fn record(&mut self) {
        self.history.push_back(self.stats());
        if self.history.len() > 1001 {
            self.history.pop_front();
        }
    }
}

pub fn gini(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let total: f64 = v.iter().sum();
    if v.is_empty() || total == 0.0 {
        return 0.0;
    }
    let n = v.len() as f64;
    let weighted: f64 = v.iter().enumerate().map(|(i, x)| (i + 1) as f64 * x).sum();
    (2.0 * weighted / (n * total) - (n + 1.0) / n).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeded_runs_are_identical_and_occupancy_is_unique() {
        for experiment in Experiment::ALL {
            let config = Config {
                experiment,
                ..Default::default()
            };
            let (mut a, mut b) = (World::new(config.clone()), World::new(config));
            for _ in 0..150 {
                a.step();
                b.step();
                assert_eq!(a.agents, b.agents);
                assert_eq!(a.cells, b.cells);
                let mut positions: Vec<_> = a.agents.iter().map(|x| x.pos).collect();
                positions.sort();
                positions.dedup();
                assert_eq!(positions.len(), a.agents.len());
                assert!(a.cells.iter().all(|c| c.sugar <= c.capacity));
                assert!(a.agents.iter().all(|a| a.sugar > 0));
                assert_eq!(
                    a.agents.len() as u64 + a.starvation_deaths + a.age_deaths + a.combat_deaths,
                    a.config.population as u64 + a.births
                );
                assert!(
                    a.agents
                        .iter()
                        .all(|agent| agent.tags < (1_u32 << a.config.culture_tags))
                );
                assert!(a.agents.iter().all(|agent| {
                    agent
                        .parents
                        .is_none_or(|[p, q]| p < agent.id && q < agent.id)
                }));
                if experiment == Experiment::Replacement {
                    assert_eq!(a.agents.len(), 400);
                }
            }
            assert_eq!(a.history, b.history);
            if experiment.social() {
                assert!(
                    a.births > 0,
                    "the default social experiment should produce offspring"
                );
            }
        }
    }
    #[test]
    fn movement_prefers_sugar_then_distance_and_harvests() {
        let mut w = World::new(Config {
            size: 10,
            population: 1,
            growback: 0,
            wrap: false,
            ..Default::default()
        });
        w.cells.iter_mut().for_each(|c| {
            c.sugar = 0;
            c.capacity = 4;
        });
        w.agents[0] = Agent {
            id: 0,
            pos: 44,
            sugar: 10,
            vision: 3,
            metabolism: 2,
            age: 0,
            max_age: 100,
            ..w.agents[0].clone()
        };
        w.cells[45].sugar = 4;
        w.cells[47].sugar = 4;
        w.cells[55].sugar = 4;
        w.step();
        assert_eq!(w.agents[0].pos, 45);
        assert_eq!(w.agents[0].sugar, 12);
        assert_eq!(w.cells[45].sugar, 0);
        assert_eq!(w.cells[55].sugar, 4); // diagonal is invisible
    }
    #[test]
    fn agent_stays_on_equal_sugar_and_cannot_enter_occupied_cell() {
        let mut w = World::new(Config {
            size: 10,
            population: 2,
            growback: 0,
            ..Default::default()
        });
        w.cells.iter_mut().for_each(|c| {
            c.sugar = 1;
            c.capacity = 4;
        });
        for (i, a) in w.agents.iter_mut().enumerate() {
            a.pos = 44 + i;
            a.metabolism = 1;
        }
        w.step();
        assert!(w.agents.iter().all(|a| a.pos == 44 || a.pos == 45));
    }
    #[test]
    fn starvation_and_replacement_age_limit() {
        let mut w = World::new(Config {
            size: 10,
            population: 1,
            growback: 0,
            ..Default::default()
        });
        w.cells.iter_mut().for_each(|c| c.sugar = 0);
        w.agents[0].sugar = 1;
        w.step();
        assert!(w.agents.is_empty());
        assert_eq!(w.starvation_deaths, 1);
        w.step();
        assert_eq!(w.stats().gini, 0.0);
        let mut w = World::new(Config {
            experiment: Experiment::Replacement,
            min_age: 1,
            max_age: 1,
            ..Default::default()
        });
        w.step();
        assert_eq!(w.agents.len(), 400);
        assert_eq!(w.births, 400);
        assert!(w.agents.iter().all(|a| a.age == 0 && a.id >= 400));
    }
    #[test]
    fn topology_growth_and_gini() {
        let w = World::new(Config::default());
        assert!(w.visible(0, 1).contains(&(49, 1)));
        let small = World::new(Config {
            size: 10,
            ..Default::default()
        });
        assert!(small.visible(0, 9).contains(&(9, 1)));
        let w = World::new(Config {
            wrap: false,
            ..Default::default()
        });
        assert_eq!(w.visible(0, 1).len(), 3);
        assert_eq!(gini(&[3.0, 3.0, 3.0]), 0.0);
        assert!((gini(&[0.0, 0.0, 9.0]) - 2.0 / 3.0).abs() < 1e-10);
        for experiment in [Experiment::Immediate, Experiment::Growback] {
            let mut w = World::new(Config {
                experiment,
                population: 1,
                ..Default::default()
            });
            w.agents.clear();
            w.cells.iter_mut().for_each(|c| c.sugar = 0);
            w.step();
            assert!(w.cells.iter().all(|c| c.sugar
                == if experiment == Experiment::Immediate {
                    c.capacity
                } else {
                    c.capacity.min(1)
                }));
        }
    }
}
