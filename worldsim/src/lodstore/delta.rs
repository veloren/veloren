use super::{
    data::{
        LodData,
        LodConfig,
    },
    index::LodIndex,
    area::LodArea,
};

/*
    A LodDelta applies a change to a Lod
    The rules for LodDeltas are strict in order to make them as simple as possible.
    A LodDelta created from LodData A can only be applied safely to another LodData equal to A.
    However LodDeltas can be combined and reverted

    I am not sure about a Vec or Hashmap, the thing is Vec is easier to fill, but might contain duplicate entries:
    E.g. change a item multiple time, bloats the Delta, with a Hashmap only the lastest state is kept.
    However i belive that most algorithms only change every Value once.
*/

#[derive(Debug, Clone)]
pub struct LodDelta<X: LodConfig> {
    pub layer0: Vec<(LodIndex, Option<X::L0>)>, // 1/16
    pub layer1: Vec<(LodIndex, Option<X::L1>)>, // 1/8
    pub layer2: Vec<(LodIndex, Option<X::L2>)>, // 1/4
    pub layer3: Vec<(LodIndex, Option<X::L3>)>, // 1/2
    pub layer4: Vec<(LodIndex, Option<X::L4>)>, // 1
    pub layer5: Vec<(LodIndex, Option<X::L5>)>, // 2
    pub layer6: Vec<(LodIndex, Option<X::L6>)>, // 4
    pub layer7: Vec<(LodIndex, Option<X::L7>)>, // 8
    pub layer8: Vec<(LodIndex, Option<X::L8>)>, // 16
    pub layer9: Vec<(LodIndex, Option<X::L9>)>, // 32
    pub layer10: Vec<(LodIndex, Option<X::L10>)>, // 64
    pub layer11: Vec<(LodIndex, Option<X::L11>)>, // 128
    pub layer12: Vec<(LodIndex, Option<X::L12>)>, // 256
    pub layer13: Vec<(LodIndex, Option<X::L13>)>, // 512
    pub layer14: Vec<(LodIndex, Option<X::L14>)>, // 1024
    pub layer15: Vec<(LodIndex, Option<X::L15>)>,  // 2048
}

impl<X: LodConfig> LodDelta<X> {
    pub fn new() -> Self {
        Self {
            layer0: Vec::new(),
            layer1: Vec::new(),
            layer2: Vec::new(),
            layer3: Vec::new(),
            layer4: Vec::new(),
            layer5: Vec::new(),
            layer6: Vec::new(),
            layer7: Vec::new(),
            layer8: Vec::new(),
            layer9: Vec::new(),
            layer10: Vec::new(),
            layer11: Vec::new(),
            layer12: Vec::new(),
            layer13: Vec::new(),
            layer14: Vec::new(),
            layer15: Vec::new(),
        }
    }

    //TODO: apply that moves out
    pub fn apply(&self, data: &mut LodData<X>) {
        for (index, item) in &self.layer15 {
            if let Some(item) = item {
                data.set15(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer14 {
            if let Some(item) = item {
                data.set14(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer13 {
            if let Some(item) = item {
                data.set13(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer12 {
            if let Some(item) = item {
                data.set12(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer11 {
            if let Some(item) = item {
                data.set11(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer10 {
            if let Some(item) = item {
                data.set10(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer9 {
            if let Some(item) = item {
                data.set9(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer8 {
            if let Some(item) = item {
                data.set8(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer7 {
            if let Some(item) = item {
                data.set7(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer6 {
            if let Some(item) = item {
                data.set6(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer5 {
            if let Some(item) = item {
                data.set5(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer4 {
            if let Some(item) = item {
                data.set4(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer3 {
            if let Some(item) = item {
                data.set3(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer2 {
            if let Some(item) = item {
                data.set2(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer1 {
            if let Some(item) = item {
                data.set1(*index, item.clone(), None);
            }
        }
        for (index, item) in &self.layer0 {
            if let Some(item) = item {
                data.set0(*index, item.clone(), None);
            }
        }
    }

    pub fn filter(&self, area: LodArea) -> Self {
        Self {
            layer0: self.layer0.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer1: self.layer1.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer2: self.layer2.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer3: self.layer3.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer4: self.layer4.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer5: self.layer5.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer6: self.layer6.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer7: self.layer7.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer8: self.layer8.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer9: self.layer9.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer10: self.layer10.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer11: self.layer11.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer12: self.layer12.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer13: self.layer13.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer14: self.layer14.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
            layer15: self.layer15.iter().filter(|(index, _)| area.is_inside(index.clone())).cloned().collect(),
        }
    }
}
