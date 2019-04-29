use crate::lodstore::Layer;
use crate::lodstore::LodLayer;

#[derive(Debug, Clone)]
pub enum Terrain {
    // 11 is max
    Unused11,
    Region9 { //512m this is for normal simulation if no player nearby
        precent_air: f32,
        percent_forrest: f32,
        percent_lava: f32,
        percent_water: f32,
    },
    Chunk5 {//32m, same detail as region, but to not force block1 everywhere in 512 area
        precent_air: f32,
        percent_forrest: f32,
        percent_lava: f32,
        percent_water: f32,
    },
    Block1 {
        material: u32,
    },
    SubBlock_4 {
        material: u32,
    },
    // -4 is min
}

impl Terrain {
    fn new() -> Self {
        Terrain::Unused11
    }
}

const LAYER5: i8 = 11;
const LAYER4: i8 = 9;
const LAYER3: i8 = 5;
const LAYER2: i8 = 0;
const LAYER1: i8 = -4;


impl Layer for Terrain {
    fn new() -> LodLayer<Terrain> {
        let mut n = LodLayer::<Terrain>::new_data(Terrain::Unused11);
        Self::drill_down(&mut n);
        n
    }

    fn get_level(layer: &LodLayer<Self>) -> i8 {
        match &layer.data {
            Terrain::Unused11 => LAYER5,
            Terrain::Region9{..} => LAYER4,
            Terrain::Chunk5{..} => LAYER3,
            Terrain::Block1{..} => LAYER2,
            Terrain::SubBlock_4{..} => -LAYER1,
        }
    }

    fn get_lower_level(layer: &LodLayer<Self>) -> Option<i8> {
        match &layer.data {
            Terrain::Unused11 => Some(LAYER4),
            Terrain::Region9{..} => Some(LAYER3),
            Terrain::Chunk5{..} => Some(LAYER2),
            Terrain::Block1{..} => Some(LAYER1),
            Terrain::SubBlock_4{..} => None,
        }
    }

    fn drill_down(layer: &mut  LodLayer<Terrain>) {
        match &layer.data {
            Terrain::Unused11 => {
                let n = LodLayer::new_data(Terrain::Region9{
                    precent_air: 1.0,
                    percent_forrest: 0.0,
                    percent_lava: 0.0,
                    percent_water: 0.0,
                });
                layer.childs = vec![n; 2_usize.pow((LAYER5-LAYER4) as u32 *3)];
            },
            Terrain::Region9{..} => {
                let n = LodLayer::new_data(Terrain::Chunk5{
                    precent_air: 1.0,
                    percent_forrest: 0.0,
                    percent_lava: 0.0,
                    percent_water: 0.0,
                });
                layer.childs = vec![n; 2_usize.pow((LAYER4-LAYER3) as u32 *3)];
            },
            Terrain::Chunk5{..} => {
                let n = LodLayer::new_data( Terrain::Block1{
                    material: 10,
                });
                layer.childs = vec![n; 2_usize.pow((LAYER3-LAYER2) as u32 *3)];
            },
            Terrain::Block1{..} => {
                let n = LodLayer::new_data( Terrain::SubBlock_4{
                    material: 10,
                });
                layer.childs = vec![n; 2_usize.pow((LAYER2-LAYER1) as u32 *3)];
            },
            Terrain::SubBlock_4{..} => {
                panic!("cannot drillDown further")
            },
        }
    }
    fn drill_up(parent: &mut LodLayer<Terrain>) {
        match &parent.data {
            Terrain::Unused11 => {
                panic!("cannot drillUp further")
            },
            Terrain::Region9{..} => {
                //recalculate values here
                parent.data = Terrain::Region9{
                    precent_air: 1.0,
                    percent_forrest: 0.0,
                    percent_lava: 0.0,
                    percent_water: 0.0,
                };
                parent.childs = vec![];
            },
            Terrain::Chunk5{..} => {
                parent.data = Terrain::Chunk5{
                    precent_air: 1.0,
                    percent_forrest: 0.0,
                    percent_lava: 0.0,
                    percent_water: 0.0,
                };
                parent.childs = vec![];
            },
            Terrain::Block1{..} => {
                parent.data = Terrain::Block1{
                    material: 10,
                };
                parent.childs = vec![];
            },
            Terrain::SubBlock_4{..} => {
                parent.data = Terrain::SubBlock_4{
                    material: 10,
                };
                parent.childs = vec![];
            },
        }
    }
}