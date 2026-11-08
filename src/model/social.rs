//! Chapter “Sex, Culture and Conflict”. Sequential movement, then culture, then births.
use super::*;
use std::collections::BTreeSet;

impl World {
    pub(super) fn step_social(&mut self) {
        for cell in &mut self.cells {
            cell.sugar = cell.capacity.min(cell.sugar + self.config.growback);
        }
        // Stable indices allow killing an agent before or after its turn without a second move.
        let mut slots: Vec<_> = std::mem::take(&mut self.agents)
            .into_iter()
            .map(Some)
            .collect();
        let mut occupied = vec![None; self.cells.len()];
        for (i, agent) in slots.iter().enumerate() {
            occupied[agent.as_ref().unwrap().pos] = Some(i);
        }
        let mut order: Vec<_> = (0..slots.len()).collect();
        self.random.shuffle(&mut order);
        for i in order {
            let Some(mut agent) = slots[i].take() else {
                continue;
            };
            occupied[agent.pos] = None;
            let destination = self.social_destination(&agent, &slots, &occupied);
            if let Some(victim_index) = occupied[destination] {
                let victim = slots[victim_index].take().unwrap();
                agent.sugar += victim.sugar.min(self.config.combat_loot as u64);
                self.combat_deaths += 1;
                occupied[destination] = None;
            }
            agent.pos = destination;
            agent.sugar += self.cells[destination].sugar as u64;
            self.cells[destination].sugar = 0;
            agent.sugar = agent.sugar.saturating_sub(agent.metabolism as u64);
            agent.age += 1;
            if agent.sugar == 0 {
                self.starvation_deaths += 1;
            } else if agent.age >= agent.max_age {
                self.age_deaths += 1;
            } else {
                occupied[destination] = Some(i);
                slots[i] = Some(agent);
            }
        }
        self.agents = slots.into_iter().flatten().collect();
        if self.config.experiment.culture() {
            self.transmit_culture();
        }
        self.reproduce();
    }

    fn social_destination(
        &mut self,
        agent: &Agent,
        slots: &[Option<Agent>],
        occupied: &[Option<usize>],
    ) -> usize {
        let fighting = self.config.experiment == Experiment::Conflict;
        let mut best = self.cells[agent.pos].sugar as u64;
        let mut distance = 0;
        let mut choices = vec![agent.pos];
        for (p, d) in self.visible(agent.pos, agent.vision).into_iter().skip(1) {
            let victim = occupied[p].and_then(|i| slots[i].as_ref());
            if let Some(victim) = victim
                && (!fighting
                    || victim.red(self.config.culture_tags) == agent.red(self.config.culture_tags)
                    || victim.sugar >= agent.sugar)
            {
                continue;
            }
            let reward = self.cells[p].sugar as u64
                + victim.map_or(0, |a| a.sugar.min(self.config.combat_loot as u64));
            // Our explicit retaliation convention: scan the attacker's axial vision at the
            // destination; a surviving richer enemy than post-harvest wealth makes it unsafe.
            if fighting
                && self.visible(p, agent.vision).iter().any(|&(q, _)| {
                    q != p
                        && occupied[q]
                            .and_then(|i| slots[i].as_ref())
                            .is_some_and(|enemy| {
                                enemy.red(self.config.culture_tags)
                                    != agent.red(self.config.culture_tags)
                                    && enemy.sugar > agent.sugar + reward
                            })
                })
            {
                continue;
            }
            if reward > best || (reward == best && d < distance) {
                best = reward;
                distance = d;
                choices.clear();
                choices.push(p);
            } else if reward == best && d == distance {
                choices.push(p);
            }
        }
        choices[self.random.below(choices.len())]
    }

    fn occupancy(&self) -> Vec<Option<usize>> {
        let mut occupied = vec![None; self.cells.len()];
        for (i, a) in self.agents.iter().enumerate() {
            occupied[a.pos] = Some(i);
        }
        occupied
    }

    fn transmit_culture(&mut self) {
        let occupied = self.occupancy();
        let mut order: Vec<_> = (0..self.agents.len()).collect();
        self.random.shuffle(&mut order);
        for i in order {
            let mut neighbors: Vec<_> = self
                .visible(self.agents[i].pos, 1)
                .into_iter()
                .skip(1)
                .filter_map(|(p, _)| occupied[p])
                .collect();
            self.random.shuffle(&mut neighbors);
            let tags = self.agents[i].tags;
            for j in neighbors {
                for _ in 0..self.config.culture_contacts {
                    let bit = 1 << self.random.below(self.config.culture_tags as usize);
                    self.agents[j].tags = (self.agents[j].tags & !bit) | (tags & bit);
                }
            }
        }
    }

    fn reproduce(&mut self) {
        let count = self.agents.len();
        let mut occupied = self.occupancy();
        let mut order: Vec<_> = (0..count).collect();
        self.random.shuffle(&mut order);
        let mut pairs = BTreeSet::new();
        for i in order {
            let mut neighbors: Vec<_> = self
                .visible(self.agents[i].pos, 1)
                .into_iter()
                .skip(1)
                .filter_map(|(p, _)| occupied[p])
                .filter(|&j| j < count)
                .collect();
            self.random.shuffle(&mut neighbors);
            for j in neighbors {
                if !pairs.insert((i.min(j), i.max(j))) {
                    continue;
                }
                let (a, b) = (&self.agents[i], &self.agents[j]);
                if a.female == b.female || !a.fertile(&self.config) || !b.fertile(&self.config) {
                    continue;
                }
                let mut free: Vec<_> = self
                    .visible(a.pos, 1)
                    .into_iter()
                    .chain(self.visible(b.pos, 1))
                    .filter_map(|(p, d)| (d == 1 && occupied[p].is_none()).then_some(p))
                    .collect();
                free.sort_unstable();
                free.dedup();
                if free.is_empty() {
                    continue;
                }
                let pos = free[self.random.below(free.len())];
                let (a, b) = (a.clone(), b.clone());
                let (cost_a, cost_b) = (a.endowment.div_ceil(2), b.endowment.div_ceil(2));
                let mut child = self.new_agent(pos);
                child.sugar = cost_a + cost_b;
                child.endowment = child.sugar;
                child.vision = if self.random.below(2) == 0 {
                    a.vision
                } else {
                    b.vision
                };
                child.metabolism = if self.random.below(2) == 0 {
                    a.metabolism
                } else {
                    b.metabolism
                };
                child.tags = 0;
                for tag in 0..self.config.culture_tags {
                    let parent = if self.random.below(2) == 0 {
                        a.tags
                    } else {
                        b.tags
                    };
                    child.tags |= parent & (1 << tag);
                }
                child.parents = Some([a.id, b.id]);
                child.generation = a.generation.max(b.generation) + 1;
                self.agents[i].sugar -= cost_a;
                self.agents[j].sugar -= cost_b;
                occupied[pos] = Some(self.agents.len());
                self.agents.push(child);
                self.births += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(experiment: Experiment) -> World {
        let mut world = World::new(Config {
            size: 10,
            population: 2,
            experiment,
            growback: 0,
            wrap: false,
            ..Default::default()
        });
        for (i, a) in world.agents.iter_mut().enumerate() {
            a.pos = 44 + i;
            a.female = i == 0;
            a.age = 20;
            a.max_age = 100;
            a.sugar = 40;
            a.endowment = 20;
            a.tags = if i == 0 { 0 } else { 2047 };
        }
        world
    }

    #[test]
    fn birth_conserves_wealth_and_inherits_genes_and_parents() {
        let mut w = pair(Experiment::Sex);
        let total: u64 = w.agents.iter().map(|a| a.sugar).sum();
        w.reproduce();
        assert_eq!(w.births, 1);
        assert_eq!(w.agents.len(), 3);
        assert_eq!(w.agents.iter().map(|a| a.sugar).sum::<u64>(), total);
        let baby = &w.agents[2];
        assert_eq!(baby.age, 0);
        assert_eq!(baby.sugar, 20);
        assert_eq!(baby.generation, 1);
        assert!(baby.parents == Some([0, 1]) || baby.parents == Some([1, 0]));
        assert!(w.agents[..2].iter().any(|a| a.vision == baby.vision));
        assert!(
            w.agents[..2]
                .iter()
                .any(|a| a.metabolism == baby.metabolism)
        );
        assert!(
            w.visible(44, 1)
                .iter()
                .chain(w.visible(45, 1).iter())
                .any(|&(p, d)| p == baby.pos && d == 1)
        );
    }

    #[test]
    fn fertility_requires_age_wealth_opposite_sex_and_space() {
        for failure in 0..5 {
            let mut w = pair(Experiment::Sex);
            match failure {
                0 => w.agents[0].age = 0,
                1 => w.agents[0].sugar = 19,
                2 => w.agents[0].female = false,
                3 => w.agents[0].age = w.config.female_fertility_end + 1,
                _ => {
                    let free: Vec<_> = w
                        .visible(44, 1)
                        .into_iter()
                        .chain(w.visible(45, 1))
                        .filter_map(|(p, d)| (d == 1 && p != 44 && p != 45).then_some(p))
                        .collect();
                    for p in free {
                        if !w.agents.iter().any(|a| a.pos == p) {
                            let mut blocker = w.new_agent(p);
                            blocker.age = 0;
                            w.agents.push(blocker);
                        }
                    }
                }
            }
            w.reproduce();
            assert_eq!(w.births, 0, "case {failure}");
        }
    }

    #[test]
    fn newborn_waits_until_next_tick_and_extinction_has_no_replacement() {
        let mut w = pair(Experiment::Sex);
        w.cells.iter_mut().for_each(|c| c.sugar = 0);
        for a in &mut w.agents {
            a.metabolism = 1;
        }
        w.step();
        assert_eq!(w.births, 1);
        let baby = w.agents.iter().find(|a| a.parents.is_some()).unwrap();
        assert_eq!(baby.age, 0);
        assert_eq!(baby.sugar, 20);
        for a in &mut w.agents {
            a.max_age = 1;
        }
        w.step();
        assert!(w.agents.is_empty());
        assert_eq!(w.births, 1);
        w.step();
        assert!(w.agents.is_empty());
    }

    #[test]
    fn culture_only_reaches_axial_neighbors_and_can_be_disabled() {
        let mut w = pair(Experiment::Culture);
        w.config.culture_tags = 1;
        w.config.culture_contacts = 1;
        w.agents[0].tags = 0;
        w.agents[1].tags = 1;
        let mut diagonal = w.new_agent(55);
        diagonal.tags = 0;
        w.agents.push(diagonal);
        // Isolate the diagonal from the second agent too.
        w.agents[2].pos = 66;
        w.transmit_culture();
        assert_eq!(w.agents[0].tags, w.agents[1].tags);
        assert_eq!(w.agents[2].tags, 0);
        w.config.culture_contacts = 0;
        w.agents[0].tags = 0;
        w.agents[1].tags = 1;
        w.transmit_culture();
        assert_ne!(w.agents[0].tags, w.agents[1].tags);
    }

    #[test]
    fn combat_obeys_tribe_wealth_loot_and_retaliation() {
        let mut w = pair(Experiment::Conflict);
        w.cells.iter_mut().for_each(|c| c.sugar = 0);
        w.agents[0].sugar = 50;
        w.agents[0].vision = 1;
        w.agents[1].sugar = 10;
        let attacker = w.agents[0].clone();
        let occupied = w.occupancy();
        let slots: Vec<_> = w.agents.iter().cloned().map(Some).collect();
        assert_eq!(w.social_destination(&attacker, &slots, &occupied), 45);
        let mut same_tribe = slots.clone();
        same_tribe[1].as_mut().unwrap().tags = 0;
        assert_eq!(w.social_destination(&attacker, &same_tribe, &occupied), 44);
        let mut rich = slots.clone();
        rich[1].as_mut().unwrap().sugar = 50;
        assert_eq!(w.social_destination(&attacker, &rich, &occupied), 44);
        let mut guard = w.new_agent(46);
        guard.tags = 2047;
        guard.sugar = 100;
        w.agents.push(guard);
        let occupied = w.occupancy();
        let slots = w.agents.iter().cloned().map(Some).collect::<Vec<_>>();
        assert_eq!(w.social_destination(&attacker, &slots, &occupied), 44);
        w.agents.pop();
        w.config.culture_contacts = 0;
        w.config.combat_loot = 3;
        for a in &mut w.agents {
            a.age = 0;
            a.vision = 1;
            a.metabolism = 1;
        }
        w.step();
        assert_eq!(w.combat_deaths, 1);
        assert_eq!(w.agents.len(), 1);
        assert_eq!(w.agents[0].id, attacker.id);
        assert_eq!(w.agents[0].sugar, 52); // 50 + capped loot 3 - metabolism 1
    }
}
