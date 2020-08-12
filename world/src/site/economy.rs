use crate::util::{DHashMap, MapVec};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Good {
    Wheat = 0,
    Flour = 1,
    Meat = 2,
    Fish = 3,
    Game = 4,
    Food = 5,
    Logs = 6,
    Wood = 7,
    Rock = 8,
    Stone = 9,
}
use Good::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Labor {
    Farmer = 0,
    Lumberjack = 1,
    Miner = 2,
    Fisher = 3,
    Hunter = 4,
    Cook = 5,
}
use Labor::*;

pub struct Economy {
    pub pop: f32,

    pub stocks: MapVec<Good, f32>,
    pub surplus: MapVec<Good, f32>,
    pub marginal_surplus: MapVec<Good, f32>,
    pub values: MapVec<Good, Option<f32>>,

    pub labor_values: MapVec<Good, Option<f32>>,
    pub material_costs: MapVec<Good, f32>,

    pub labors: MapVec<Labor, f32>,
    pub yields: MapVec<Labor, f32>,
    pub productivity: MapVec<Labor, f32>,
}

impl Default for Economy {
    fn default() -> Self {
        Self {
            pop: 32.0,

            stocks: Default::default(),
            surplus: Default::default(),
            marginal_surplus: Default::default(),
            values: Default::default(),

            labor_values: Default::default(),
            material_costs: Default::default(),

            labors: Default::default(),
            yields: Default::default(),
            productivity: Default::default(),
        }
    }
}

impl Economy {
    pub fn get_orders(&self) -> DHashMap<Option<Labor>, Vec<(Good, f32)>> {
        vec![
            (None, vec![(Food, 0.5)]),
            (Some(Cook), vec![
                (Flour, 12.0),
                (Meat, 4.0),
                (Wood, 1.5),
                (Stone, 1.0),
            ]),
            (Some(Lumberjack), vec![(Logs, 0.5)]),
            (Some(Miner), vec![(Rock, 0.5)]),
            (Some(Fisher), vec![(Fish, 4.0)]),
            (Some(Hunter), vec![(Game, 1.0)]),
            (Some(Farmer), vec![(Wheat, 2.0)]),
        ]
        .into_iter()
        .collect()
    }

    pub fn get_productivity(&self) -> MapVec<Labor, (Good, f32)> {
        // Per labourer, per year
        MapVec::from_list(
            &[
                (Farmer, (Flour, 2.0)),
                (Lumberjack, (Wood, 0.5)),
                (Miner, (Stone, 0.5)),
                (Fisher, (Meat, 4.0)),
                (Hunter, (Meat, 1.0)),
                (Cook, (Food, 16.0)),
            ],
            (Rock, 0.0),
        )
        .map(|l, (good, v)| (good, v * (1.0 + self.labors[l])))
    }

    pub fn replenish(&mut self, time: f32) {
        //use rand::Rng;
        for (i, (g, v)) in [
            (Wheat, 50.0),
            (Logs, 20.0),
            (Rock, 120.0),
            (Game, 12.0),
            (Fish, 10.0),
        ]
        .iter()
        .enumerate()
        {
            self.stocks[*g] = (*v
                * (1.25 + (((time * 0.0001 + i as f32).sin() + 1.0) % 1.0) * 0.5)
                - self.stocks[*g])
                * 0.075; //rand::thread_rng().gen_range(0.05, 0.1);
        }
    }
}

impl Default for Good {
    fn default() -> Self {
        Good::Rock // Arbitrary
    }
}

impl Good {
    pub fn list() -> &'static [Self] {
        static GOODS: [Good; 10] = [
            Wheat, Flour, Meat, Fish, Game, Food, Logs, Wood, Rock, Stone,
        ];

        &GOODS
    }

    pub fn decay_rate(&self) -> f32 {
        match self {
            Food => 0.2,
            Wheat => 0.1,
            Meat => 0.25,
            Fish => 0.2,
            _ => 0.0,
        }
    }
}

impl Labor {
    pub fn list() -> &'static [Self] {
        static LABORS: [Labor; 6] = [Farmer, Lumberjack, Miner, Fisher, Hunter, Cook];

        &LABORS
    }
}
