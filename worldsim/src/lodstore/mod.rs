pub mod index;
use std::sync::Arc;
use std::collections::HashMap;
use vek::*;
use index::{
    LodIndex,
    relative_to_1d,
};

/*
LOD Data contains different Entries in different vecs, every entry has a "pointer" to it's child start.
This is the structure to store a region and all subscribed information
*/
pub trait LayerInfo {
    fn get_child_index(&self) -> Option<usize>;
    const layer_volume: Vec3<u32>; // e.g. (1|1|1) for l0 or (4|4|4) for l2 optimization
    const child_layer_id: Option<u8>;
    const child_len: usize; //number of childs on this layer, MUST BE 2^(SELF::child_dim*3)
}

/* for dyn trait objects, not really fast, but faster to code, use as a makeshift solution only! */
pub trait LayerInfoDyn {
    fn get_child_index(&self) -> Option<usize>;
    fn get_layer_volume(&self) -> Vec3<u32>;
    fn get_child_layer_id(&self) -> Option<u8>;
    fn get_child_len(&self) -> usize;
}

impl<L: LayerInfo> LayerInfoDyn for L {
    fn get_child_index(&self) -> Option<usize> {
        self.get_child_index()
    }
    fn get_layer_volume(&self) -> Vec3<u32> {
        L::layer_volume
    }
    fn get_child_layer_id(&self) -> Option<u8> {
        L::child_layer_id
    }
    fn get_child_len(&self) -> usize {
        L::child_len
    }
}



pub trait LodConfig {
    type L0: LayerInfo; // 2^-4
    type L1: LayerInfo;
    type L2: LayerInfo;
    type L3: LayerInfo;
    type L4: LayerInfo; // 2^0
    type L5: LayerInfo;
    type L6: LayerInfo;
    type L7: LayerInfo;
    type L8: LayerInfo;
    type L9: LayerInfo;
    type L10: LayerInfo;
    type L11: LayerInfo;
    type L12: LayerInfo;
    type L13: LayerInfo;
    type L14: LayerInfo;
    type L15: LayerInfo; // 2^11

    const anchor_layer_id: u8;

    fn setup(&mut self);
    fn drill_down(data: &mut LodData::<Self>, level: u8, index: usize) where Self: Sized;
    fn drill_up(data: &mut LodData::<Self>, level: u8, parent_index: usize) where Self: Sized;
}

/*
There is another optimization problem: We have OWNED data and foreign DATA in the struct, but we don't want the foreign data to take a lot of space if unused
But both needs to be accessible transparent without overhead in calculation, difficult.
Imagine a Terrain, which top level is L13, so it would have 64 entries for the owned and 1664 for foreign data if everything is filled.
So we really only fill the boarder giving us 152 border areas.
One could think about multiple entry levels for foreign and owned data, but that means, that foreign data without a parent would exist, which might break algorithms....
So for now we go with a single anchorlevel for now, and hope the designer chooses good levels
*/

//ERROR NEXT STEP IS TO WORK ON SYSTEMS IN ORDER TO KNOW WHAT EXACTLY WE NEED. BUILD A FAKE "RASTERIZER" E:G:

#[derive(Debug, Clone)]
pub struct LodData<X: LodConfig> {
    pub layer0: Vec<X::L0>, // 1/16
    pub layer1: Vec<X::L1>, // 1/8
    pub layer2: Vec<X::L2>, // 1/4
    pub layer3: Vec<X::L3>, // 1/2
    pub layer4: Vec<X::L4>, // 1
    pub layer5: Vec<X::L5>, // 2
    pub layer6: Vec<X::L6>, // 4
    pub layer7: Vec<X::L7>, // 8
    pub layer8: Vec<X::L8>, // 16
    pub layer9: Vec<X::L9>, // 32
    pub layer10: Vec<X::L10>, // 64
    pub layer11: Vec<X::L11>, // 128
    pub layer12: Vec<X::L12>, // 256
    pub layer13: Vec<X::L13>, // 512
    pub layer14: Vec<X::L14>, // 1024
    pub layer15: Vec<X::L15>,  // 2048
    pub anchor: HashMap<LodIndex, usize>,
}

impl<X: LodConfig> LodData<X>
{
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
            anchor: HashMap::new(),
        }
    }

    /*
    Da fuq is his code you might ask,
    but seriosly. because of logic reasons you have to know the level you want anyway, so we go for it ;)

    int_getN => if you know the parent and absolute index, as well as parents absolut index, i return you your child
    */

    fn int_get0<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L0 {
        debug_assert_eq!(T::child_layer_id, X::L0::child_layer_id);
        &self.layer0[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get1<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L1 {
        debug_assert_eq!(T::child_layer_id, X::L1::child_layer_id);
        &self.layer1[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get2<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L2 {
        debug_assert_eq!(T::child_layer_id, X::L2::child_layer_id);
        &self.layer2[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get3<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L3 {
        debug_assert_eq!(T::child_layer_id, X::L3::child_layer_id);
        &self.layer3[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get4<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L4 {
        debug_assert_eq!(T::child_layer_id, X::L4::child_layer_id);
        &self.layer4[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get5<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L5 {
        debug_assert_eq!(T::child_layer_id, X::L5::child_layer_id);
        &self.layer5[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get6<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L6 {
        debug_assert_eq!(T::child_layer_id, X::L6::child_layer_id);
        &self.layer6[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get7<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L7 {
        debug_assert_eq!(T::child_layer_id, X::L7::child_layer_id);
        &self.layer7[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get8<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L8 {
        debug_assert_eq!(T::child_layer_id, X::L8::child_layer_id);
        &self.layer8[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get9<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L9 {
        debug_assert_eq!(T::child_layer_id, X::L9::child_layer_id);
        &self.layer9[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get10<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L10 {
        debug_assert_eq!(T::child_layer_id, X::L10::child_layer_id);
        &self.layer10[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get11<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L11 {
        debug_assert_eq!(T::child_layer_id, X::L11::child_layer_id);
        &self.layer11[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get12<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L12 {
        debug_assert_eq!(T::child_layer_id, X::L12::child_layer_id);
        &self.layer12[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get13<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L13 {
        debug_assert_eq!(T::child_layer_id, X::L13::child_layer_id);
        &self.layer13[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    fn int_get14<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L14 {
        debug_assert_eq!(T::child_layer_id, X::L14::child_layer_id);
        &self.layer14[relative_to_1d(index - parent_index, T::layer_volume)]
    }

    /*
    These matches are const evaluatable, hope for the optimizer
    */

    fn int_hop_get14<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L14 {
        match T::child_layer_id {
            Some(14) => {
                self.int_get14(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get13<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L13 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get13(l14, index, parent_index)
            },
            Some(13) => {
                self.int_get13(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get12<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L12 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get12(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get12(l13, index, parent_index)
            },
            Some(12) => {
                self.int_get12(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get11<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L11 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get11(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get11(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get11(l12, index, parent_index)
            },
            Some(11) => {
                self.int_get11(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get10<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L10 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get10(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get10(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get10(l12, index, parent_index)
            },
            Some(11) => {
                let l11 = self.int_get11(parent, index, parent_index);
                self.int_hop_get10(l11, index, parent_index)
            },
            Some(10) => {
                self.int_get10(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get9<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L9 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get9(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get9(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get9(l12, index, parent_index)
            },
            Some(11) => {
                let l11 = self.int_get11(parent, index, parent_index);
                self.int_hop_get9(l11, index, parent_index)
            },
            Some(10) => {
                let l10 = self.int_get10(parent, index, parent_index);
                self.int_hop_get9(l10, index, parent_index)
            },
            Some(9) => {
                self.int_get9(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get8<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L8 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get8(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get8(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get8(l12, index, parent_index)
            },
            Some(11) => {
                let l11 = self.int_get11(parent, index, parent_index);
                self.int_hop_get8(l11, index, parent_index)
            },
            Some(10) => {
                let l10 = self.int_get10(parent, index, parent_index);
                self.int_hop_get8(l10, index, parent_index)
            },
            Some(9) => {
                let l9 = self.int_get9(parent, index, parent_index);
                self.int_hop_get8(l9, index, parent_index)
            },
            Some(8) => {
                self.int_get8(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get7<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L7 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get7(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get7(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get7(l12, index, parent_index)
            },
            Some(11) => {
                let l11 = self.int_get11(parent, index, parent_index);
                self.int_hop_get7(l11, index, parent_index)
            },
            Some(10) => {
                let l10 = self.int_get10(parent, index, parent_index);
                self.int_hop_get7(l10, index, parent_index)
            },
            Some(9) => {
                let l9 = self.int_get9(parent, index, parent_index);
                self.int_hop_get7(l9, index, parent_index)
            },
            Some(8) => {
                let l8 = self.int_get8(parent, index, parent_index);
                self.int_hop_get7(l8, index, parent_index)
            },
            Some(7) => {
                self.int_get7(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get6<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L6 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get6(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get6(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get6(l12, index, parent_index)
            },
            Some(11) => {
                let l11 = self.int_get11(parent, index, parent_index);
                self.int_hop_get6(l11, index, parent_index)
            },
            Some(10) => {
                let l10 = self.int_get10(parent, index, parent_index);
                self.int_hop_get6(l10, index, parent_index)
            },
            Some(9) => {
                let l9 = self.int_get9(parent, index, parent_index);
                self.int_hop_get6(l9, index, parent_index)
            },
            Some(8) => {
                let l8 = self.int_get8(parent, index, parent_index);
                self.int_hop_get6(l8, index, parent_index)
            },
            Some(7) => {
                let l7 = self.int_get7(parent, index, parent_index);
                self.int_hop_get6(l7, index, parent_index)
            },
            Some(6) => {
                self.int_get6(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get5<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L5 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get5(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get5(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get5(l12, index, parent_index)
            },
            Some(11) => {
                let l11 = self.int_get11(parent, index, parent_index);
                self.int_hop_get5(l11, index, parent_index)
            },
            Some(10) => {
                let l10 = self.int_get10(parent, index, parent_index);
                self.int_hop_get5(l10, index, parent_index)
            },
            Some(9) => {
                let l9 = self.int_get9(parent, index, parent_index);
                self.int_hop_get5(l9, index, parent_index)
            },
            Some(8) => {
                let l8 = self.int_get8(parent, index, parent_index);
                self.int_hop_get5(l8, index, parent_index)
            },
            Some(7) => {
                let l7 = self.int_get7(parent, index, parent_index);
                self.int_hop_get5(l7, index, parent_index)
            },
            Some(6) => {
                let l6 = self.int_get6(parent, index, parent_index);
                self.int_hop_get5(l6, index, parent_index)
            },
            Some(5) => {
                self.int_get5(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get4<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L4 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get4(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get4(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get4(l12, index, parent_index)
            },
            Some(11) => {
                let l11 = self.int_get11(parent, index, parent_index);
                self.int_hop_get4(l11, index, parent_index)
            },
            Some(10) => {
                let l10 = self.int_get10(parent, index, parent_index);
                self.int_hop_get4(l10, index, parent_index)
            },
            Some(9) => {
                let l9 = self.int_get9(parent, index, parent_index);
                self.int_hop_get4(l9, index, parent_index)
            },
            Some(8) => {
                let l8 = self.int_get8(parent, index, parent_index);
                self.int_hop_get4(l8, index, parent_index)
            },
            Some(7) => {
                let l7 = self.int_get7(parent, index, parent_index);
                self.int_hop_get4(l7, index, parent_index)
            },
            Some(6) => {
                let l6 = self.int_get6(parent, index, parent_index);
                self.int_hop_get4(l6, index, parent_index)
            },
            Some(5) => {
                let l5 = self.int_get5(parent, index, parent_index);
                self.int_hop_get4(l5, index, parent_index)
            },
            Some(4) => {
                self.int_get4(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get3<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L3 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get3(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get3(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get3(l12, index, parent_index)
            },
            Some(11) => {
                let l11 = self.int_get11(parent, index, parent_index);
                self.int_hop_get3(l11, index, parent_index)
            },
            Some(10) => {
                let l10 = self.int_get10(parent, index, parent_index);
                self.int_hop_get3(l10, index, parent_index)
            },
            Some(9) => {
                let l9 = self.int_get9(parent, index, parent_index);
                self.int_hop_get3(l9, index, parent_index)
            },
            Some(8) => {
                let l8 = self.int_get8(parent, index, parent_index);
                self.int_hop_get3(l8, index, parent_index)
            },
            Some(7) => {
                let l7 = self.int_get7(parent, index, parent_index);
                self.int_hop_get3(l7, index, parent_index)
            },
            Some(6) => {
                let l6 = self.int_get6(parent, index, parent_index);
                self.int_hop_get3(l6, index, parent_index)
            }
            Some(5) => {
                let l5 = self.int_get5(parent, index, parent_index);
                self.int_hop_get3(l5, index, parent_index)
            },
            Some(4) => {
                let l4 = self.int_get4(parent, index, parent_index);
                self.int_hop_get3(l4, index, parent_index)
            },
            Some(3) => {
                self.int_get3(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get2<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L2 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get2(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get2(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get2(l12, index, parent_index)
            },
            Some(11) => {
                let l11 = self.int_get11(parent, index, parent_index);
                self.int_hop_get2(l11, index, parent_index)
            },
            Some(10) => {
                let l10 = self.int_get10(parent, index, parent_index);
                self.int_hop_get2(l10, index, parent_index)
            },
            Some(9) => {
                let l9 = self.int_get9(parent, index, parent_index);
                self.int_hop_get2(l9, index, parent_index)
            },
            Some(8) => {
                let l8 = self.int_get8(parent, index, parent_index);
                self.int_hop_get2(l8, index, parent_index)
            },
            Some(7) => {
                let l7 = self.int_get7(parent, index, parent_index);
                self.int_hop_get2(l7, index, parent_index)
            },
            Some(6) => {
                let l6 = self.int_get6(parent, index, parent_index);
                self.int_hop_get2(l6, index, parent_index)
            },
            Some(5) => {
                let l6 = self.int_get6(parent, index, parent_index);
                self.int_hop_get2(l6, index, parent_index)
            },
            Some(4) => {
                let l6 = self.int_get6(parent, index, parent_index);
                self.int_hop_get2(l6, index, parent_index)
            },
            Some(3) => {
                let l6 = self.int_get6(parent, index, parent_index);
                self.int_hop_get2(l6, index, parent_index)
            },
            Some(2) => {
                self.int_get2(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get1<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L1 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get1(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get1(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get1(l12, index, parent_index)
            },
            Some(11) => {
                let l11 = self.int_get11(parent, index, parent_index);
                self.int_hop_get1(l11, index, parent_index)
            },
            Some(10) => {
                let l10 = self.int_get10(parent, index, parent_index);
                self.int_hop_get1(l10, index, parent_index)
            },
            Some(9) => {
                let l9 = self.int_get9(parent, index, parent_index);
                self.int_hop_get1(l9, index, parent_index)
            },
            Some(8) => {
                let l8 = self.int_get8(parent, index, parent_index);
                self.int_hop_get1(l8, index, parent_index)
            },
            Some(7) => {
                let l7 = self.int_get7(parent, index, parent_index);
                self.int_hop_get1(l7, index, parent_index)
            },
            Some(6) => {
                let l6 = self.int_get6(parent, index, parent_index);
                self.int_hop_get1(l6, index, parent_index)
            },
            Some(5) => {
                let l5 = self.int_get5(parent, index, parent_index);
                self.int_hop_get1(l5, index, parent_index)
            },
            Some(4) => {
                let l4 = self.int_get4(parent, index, parent_index);
                self.int_hop_get1(l4, index, parent_index)
            },
            Some(3) => {
                let l3 = self.int_get3(parent, index, parent_index);
                self.int_hop_get1(l3, index, parent_index)
            },
            Some(2) => {
                let l2 = self.int_get2(parent, index, parent_index);
                self.int_hop_get1(l2, index, parent_index)
            },
            Some(1) => {
                self.int_get1(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }

    fn int_hop_get0<T: LayerInfo>(&self, parent: &T, index: LodIndex, parent_index: LodIndex) -> &X::L0 {
        match T::child_layer_id {
            Some(14) => {
                let l14 = self.int_get14(parent, index, parent_index);
                self.int_hop_get0(l14, index, parent_index)
            },
            Some(13) => {
                let l13 = self.int_get13(parent, index, parent_index);
                self.int_hop_get0(l13, index, parent_index)
            },
            Some(12) => {
                let l12 = self.int_get12(parent, index, parent_index);
                self.int_hop_get0(l12, index, parent_index)
            },
            Some(11) => {
                let l11 = self.int_get11(parent, index, parent_index);
                self.int_hop_get0(l11, index, parent_index)
            },
            Some(10) => {
                let l10 = self.int_get10(parent, index, parent_index);
                self.int_hop_get0(l10, index, parent_index)
            },
            Some(9) => {
                let l9 = self.int_get9(parent, index, parent_index);
                self.int_hop_get0(l9, index, parent_index)
            },
            Some(8) => {
                let l8 = self.int_get8(parent, index, parent_index);
                self.int_hop_get0(l8, index, parent_index)
            },
            Some(7) => {
                let l7 = self.int_get7(parent, index, parent_index);
                self.int_hop_get0(l7, index, parent_index)
            },
            Some(6) => {
                let l6 = self.int_get6(parent, index, parent_index);
                self.int_hop_get0(l6, index, parent_index)
            },
            Some(5) => {
                let l5 = self.int_get5(parent, index, parent_index);
                self.int_hop_get0(l5, index, parent_index)
            },
            Some(4) => {
                let l4 = self.int_get4(parent, index, parent_index);
                self.int_hop_get0(l4, index, parent_index)
            },
            Some(3) => {
                let l3 = self.int_get3(parent, index, parent_index);
                self.int_hop_get0(l3, index, parent_index)
            },
            Some(2) => {
                let l2 = self.int_get2(parent, index, parent_index);
                self.int_hop_get0(l2, index, parent_index)
            },
            Some(1) => {
                let l1 = self.int_get1(parent, index, parent_index);
                self.int_hop_get0(l1, index, parent_index)
            },
            Some(0) => {
                self.int_get0(parent, index, parent_index)
            },
            _ => unreachable!("wrong layer info"),
        }
    }






    pub fn get15(&self, index: LodIndex) -> &X::L15 {
        debug_assert!(Some(X::anchor_layer_id) > X::L15::child_layer_id);
        &self.layer15[0]
    }

    pub fn get14(&self, index: LodIndex) -> &X::L14 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get14(l15, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L14::child_layer_id);
            &self.layer14[0]
        }
    }

    pub fn get13(&self, index: LodIndex) -> &X::L13 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get13(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get13(l14, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L13::child_layer_id);
            &self.layer13[0]
        }
    }

    pub fn get12(&self, index: LodIndex) -> &X::L12 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get12(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get12(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get12(l13, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L12::child_layer_id);
            &self.layer12[0]
        }
    }

    pub fn get11(&self, index: LodIndex) -> &X::L11 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get11(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get11(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get11(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get11(l12, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L11::child_layer_id);
            &self.layer11[0]
        }
    }

    pub fn get10(&self, index: LodIndex) -> &X::L10 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get10(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get10(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get10(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get10(l12, index, index)
        } else if Some(X::anchor_layer_id) == X::L11::child_layer_id {
            let l11 = self.get11(index);
            self.int_hop_get10(l11, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L10::child_layer_id);
            &self.layer10[0]
        }
    }

    pub fn get9(&self, index: LodIndex) -> &X::L9 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get9(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get9(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get9(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get9(l12, index, index)
        } else if Some(X::anchor_layer_id) == X::L11::child_layer_id {
            let l11 = self.get11(index);
            self.int_hop_get9(l11, index, index)
        } else if Some(X::anchor_layer_id) == X::L10::child_layer_id {
            let l10 = self.get10(index);
            self.int_hop_get9(l10, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L9::child_layer_id);
            &self.layer9[0]
        }
    }

    pub fn get8(&self, index: LodIndex) -> &X::L8 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get8(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get8(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get8(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get8(l12, index, index)
        } else if Some(X::anchor_layer_id) == X::L11::child_layer_id {
            let l11 = self.get11(index);
            self.int_hop_get8(l11, index, index)
        } else if Some(X::anchor_layer_id) == X::L10::child_layer_id {
            let l10 = self.get10(index);
            self.int_hop_get8(l10, index, index)
        } else if Some(X::anchor_layer_id) == X::L9::child_layer_id {
            let l9 = self.get9(index);
            self.int_hop_get8(l9, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L8::child_layer_id);
            &self.layer8[0]
        }
    }

    pub fn get7(&self, index: LodIndex) -> &X::L7 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get7(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get7(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get7(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get7(l12, index, index)
        } else if Some(X::anchor_layer_id) == X::L11::child_layer_id {
            let l11 = self.get11(index);
            self.int_hop_get7(l11, index, index)
        } else if Some(X::anchor_layer_id) == X::L10::child_layer_id {
            let l10 = self.get10(index);
            self.int_hop_get7(l10, index, index)
        } else if Some(X::anchor_layer_id) == X::L9::child_layer_id {
            let l9 = self.get9(index);
            self.int_hop_get7(l9, index, index)
        } else if Some(X::anchor_layer_id) == X::L8::child_layer_id {
            let l8 = self.get8(index);
            self.int_hop_get7(l8, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L7::child_layer_id);
            &self.layer7[0]
        }
    }

    pub fn get6(&self, index: LodIndex) -> &X::L6 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get6(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get6(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get6(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get6(l12, index, index)
        } else if Some(X::anchor_layer_id) == X::L11::child_layer_id {
            let l11 = self.get11(index);
            self.int_hop_get6(l11, index, index)
        } else if Some(X::anchor_layer_id) == X::L10::child_layer_id {
            let l10 = self.get10(index);
            self.int_hop_get6(l10, index, index)
        } else if Some(X::anchor_layer_id) == X::L9::child_layer_id {
            let l9 = self.get9(index);
            self.int_hop_get6(l9, index, index)
        } else if Some(X::anchor_layer_id) == X::L8::child_layer_id {
            let l8 = self.get8(index);
            self.int_hop_get6(l8, index, index)
        } else if Some(X::anchor_layer_id) == X::L7::child_layer_id {
            let l7 = self.get7(index);
            self.int_hop_get6(l7, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L6::child_layer_id);
            &self.layer6[0]
        }
    }

    pub fn get5(&self, index: LodIndex) -> &X::L5 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get5(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get5(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get5(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get5(l12, index, index)
        } else if Some(X::anchor_layer_id) == X::L11::child_layer_id {
            let l11 = self.get11(index);
            self.int_hop_get5(l11, index, index)
        } else if Some(X::anchor_layer_id) == X::L10::child_layer_id {
            let l10 = self.get10(index);
            self.int_hop_get5(l10, index, index)
        } else if Some(X::anchor_layer_id) == X::L9::child_layer_id {
            let l9 = self.get9(index);
            self.int_hop_get5(l9, index, index)
        } else if Some(X::anchor_layer_id) == X::L8::child_layer_id {
            let l8 = self.get8(index);
            self.int_hop_get5(l8, index, index)
        } else if Some(X::anchor_layer_id) == X::L7::child_layer_id {
            let l7 = self.get7(index);
            self.int_hop_get5(l7, index, index)
        } else if Some(X::anchor_layer_id) == X::L6::child_layer_id {
            let l6 = self.get6(index);
            self.int_hop_get5(l6, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L5::child_layer_id);
            &self.layer5[0]
        }
    }

    pub fn get4(&self, index: LodIndex) -> &X::L4 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get4(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get4(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get4(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get4(l12, index, index)
        } else if Some(X::anchor_layer_id) == X::L11::child_layer_id {
            let l11 = self.get11(index);
            self.int_hop_get4(l11, index, index)
        } else if Some(X::anchor_layer_id) == X::L10::child_layer_id {
            let l10 = self.get10(index);
            self.int_hop_get4(l10, index, index)
        } else if Some(X::anchor_layer_id) == X::L9::child_layer_id {
            let l9 = self.get9(index);
            self.int_hop_get4(l9, index, index)
        } else if Some(X::anchor_layer_id) == X::L8::child_layer_id {
            let l8 = self.get8(index);
            self.int_hop_get4(l8, index, index)
        } else if Some(X::anchor_layer_id) == X::L7::child_layer_id {
            let l7 = self.get7(index);
            self.int_hop_get4(l7, index, index)
        } else if Some(X::anchor_layer_id) == X::L6::child_layer_id {
            let l6 = self.get6(index);
            self.int_hop_get4(l6, index, index)
        } else if Some(X::anchor_layer_id) == X::L5::child_layer_id {
            let l5 = self.get5(index);
            self.int_hop_get4(l5, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L4::child_layer_id);
            &self.layer4[0]
        }
    }

    pub fn get3(&self, index: LodIndex) -> &X::L3 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get3(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get3(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get3(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get3(l12, index, index)
        } else if Some(X::anchor_layer_id) == X::L11::child_layer_id {
            let l11 = self.get11(index);
            self.int_hop_get3(l11, index, index)
        } else if Some(X::anchor_layer_id) == X::L10::child_layer_id {
            let l10 = self.get10(index);
            self.int_hop_get3(l10, index, index)
        } else if Some(X::anchor_layer_id) == X::L9::child_layer_id {
            let l9 = self.get9(index);
            self.int_hop_get3(l9, index, index)
        } else if Some(X::anchor_layer_id) == X::L8::child_layer_id {
            let l8 = self.get8(index);
            self.int_hop_get3(l8, index, index)
        } else if Some(X::anchor_layer_id) == X::L7::child_layer_id {
            let l7 = self.get7(index);
            self.int_hop_get3(l7, index, index)
        } else if Some(X::anchor_layer_id) == X::L6::child_layer_id {
            let l6 = self.get6(index);
            self.int_hop_get3(l6, index, index)
        } else if Some(X::anchor_layer_id) == X::L5::child_layer_id {
            let l5 = self.get5(index);
            self.int_hop_get3(l5, index, index)
        } else if Some(X::anchor_layer_id) == X::L4::child_layer_id {
            let l4 = self.get4(index);
            self.int_hop_get3(l4, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L3::child_layer_id);
            &self.layer3[0]
        }
    }

    pub fn get2(&self, index: LodIndex) -> &X::L2 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get2(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get2(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get2(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get2(l12, index, index)
        } else if Some(X::anchor_layer_id) == X::L11::child_layer_id {
            let l11 = self.get11(index);
            self.int_hop_get2(l11, index, index)
        } else if Some(X::anchor_layer_id) == X::L10::child_layer_id {
            let l10 = self.get10(index);
            self.int_hop_get2(l10, index, index)
        } else if Some(X::anchor_layer_id) == X::L9::child_layer_id {
            let l9 = self.get9(index);
            self.int_hop_get2(l9, index, index)
        } else if Some(X::anchor_layer_id) == X::L8::child_layer_id {
            let l8 = self.get8(index);
            self.int_hop_get2(l8, index, index)
        } else if Some(X::anchor_layer_id) == X::L7::child_layer_id {
            let l7 = self.get7(index);
            self.int_hop_get2(l7, index, index)
        } else if Some(X::anchor_layer_id) == X::L6::child_layer_id {
            let l6 = self.get6(index);
            self.int_hop_get2(l6, index, index)
        } else if Some(X::anchor_layer_id) == X::L5::child_layer_id {
            let l5 = self.get5(index);
            self.int_hop_get2(l5, index, index)
        } else if Some(X::anchor_layer_id) == X::L4::child_layer_id {
            let l4 = self.get4(index);
            self.int_hop_get2(l4, index, index)
        } else if Some(X::anchor_layer_id) == X::L3::child_layer_id {
            let l3 = self.get3(index);
            self.int_hop_get2(l3, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L2::child_layer_id);
            &self.layer2[0]
        }
    }

    pub fn get1(&self, index: LodIndex) -> &X::L1 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get1(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get1(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get1(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get1(l12, index, index)
        } else if Some(X::anchor_layer_id) == X::L11::child_layer_id {
            let l11 = self.get11(index);
            self.int_hop_get1(l11, index, index)
        } else if Some(X::anchor_layer_id) == X::L10::child_layer_id {
            let l10 = self.get10(index);
            self.int_hop_get1(l10, index, index)
        } else if Some(X::anchor_layer_id) == X::L9::child_layer_id {
            let l9 = self.get9(index);
            self.int_hop_get1(l9, index, index)
        } else if Some(X::anchor_layer_id) == X::L8::child_layer_id {
            let l8 = self.get8(index);
            self.int_hop_get1(l8, index, index)
        } else if Some(X::anchor_layer_id) == X::L7::child_layer_id {
            let l7 = self.get7(index);
            self.int_hop_get1(l7, index, index)
        } else if Some(X::anchor_layer_id) == X::L6::child_layer_id {
            let l6 = self.get6(index);
            self.int_hop_get1(l6, index, index)
        } else if Some(X::anchor_layer_id) == X::L5::child_layer_id {
            let l5 = self.get5(index);
            self.int_hop_get1(l5, index, index)
        } else if Some(X::anchor_layer_id) == X::L4::child_layer_id {
            let l4 = self.get4(index);
            self.int_hop_get1(l4, index, index)
        } else if Some(X::anchor_layer_id) == X::L3::child_layer_id {
            let l3 = self.get3(index);
            self.int_hop_get1(l3, index, index)
        } else if Some(X::anchor_layer_id) == X::L2::child_layer_id {
            let l2 = self.get2(index);
            self.int_hop_get1(l2, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L1::child_layer_id);
            &self.layer1[0]
        }
    }

    pub fn get0(&self, index: LodIndex) -> &X::L0 {
        if Some(X::anchor_layer_id) == X::L15::child_layer_id {
            let l15 = self.get15(index);
            self.int_hop_get0(l15, index, index)
        } else if Some(X::anchor_layer_id) == X::L14::child_layer_id {
            let l14 = self.get14(index);
            self.int_hop_get0(l14, index, index)
        } else if Some(X::anchor_layer_id) == X::L13::child_layer_id {
            let l13 = self.get13(index);
            self.int_hop_get0(l13, index, index)
        } else if Some(X::anchor_layer_id) == X::L12::child_layer_id {
            let l12 = self.get12(index);
            self.int_hop_get0(l12, index, index)
        } else if Some(X::anchor_layer_id) == X::L11::child_layer_id {
            let l11 = self.get11(index);
            self.int_hop_get0(l11, index, index)
        } else if Some(X::anchor_layer_id) == X::L10::child_layer_id {
            let l10 = self.get10(index);
            self.int_hop_get0(l10, index, index)
        } else if Some(X::anchor_layer_id) == X::L9::child_layer_id {
            let l9 = self.get9(index);
            self.int_hop_get0(l9, index, index)
        } else if Some(X::anchor_layer_id) == X::L8::child_layer_id {
            let l8 = self.get8(index);
            self.int_hop_get0(l8, index, index)
        } else if Some(X::anchor_layer_id) == X::L7::child_layer_id {
            let l7 = self.get7(index);
            self.int_hop_get0(l7, index, index)
        } else if Some(X::anchor_layer_id) == X::L6::child_layer_id {
            let l6 = self.get6(index);
            self.int_hop_get0(l6, index, index)
        } else if Some(X::anchor_layer_id) == X::L5::child_layer_id {
            let l5 = self.get5(index);
            self.int_hop_get0(l5, index, index)
        } else if Some(X::anchor_layer_id) == X::L4::child_layer_id {
            let l4 = self.get4(index);
            self.int_hop_get0(l4, index, index)
        } else if Some(X::anchor_layer_id) == X::L3::child_layer_id {
            let l3 = self.get3(index);
            self.int_hop_get0(l3, index, index)
        } else if Some(X::anchor_layer_id) == X::L2::child_layer_id {
            let l2 = self.get2(index);
            self.int_hop_get0(l2, index, index)
        } else if Some(X::anchor_layer_id) == X::L1::child_layer_id {
            let l1 = self.get1(index);
            self.int_hop_get0(l1, index, index)
        } else {
            debug_assert!(Some(X::anchor_layer_id) > X::L0::child_layer_id);
            &self.layer0[0]
        }
    }

    /*
        function to return a trait object, should not be used because slow,
        so as a rule of thumb only use it in case i modify self, because modify should occur not that often, and is slow anyways
    */
    fn get_mut_dyn(&mut self, level: u8, i: usize) -> &mut dyn LayerInfoDyn {
        match level {
            0 => &mut self.layer0[i],
            1 => &mut self.layer1[i],
            2 => &mut self.layer2[i],
            3 => &mut self.layer3[i],
            4 => &mut self.layer4[i],
            5 => &mut self.layer5[i],
            6 => &mut self.layer6[i],
            7 => &mut self.layer7[i],
            8 => &mut self.layer8[i],
            9 => &mut self.layer9[i],
            10 => &mut self.layer10[i],
            11 => &mut self.layer11[i],
            12 => &mut self.layer12[i],
            13 => &mut self.layer13[i],
            14 => &mut self.layer14[i],
            15 => &mut self.layer15[i],
            _ => panic!("invalid level"),
        }
    }

    /*
    These functions allow you to make the LodLayer provide a certain LOD for the specified area
    */
    /*is at least minimum or maximum*/
    pub fn make_at_least(&mut self, lower: LodIndex, upper: LodIndex, level: u8) {
        //ERROR, DOES NOT RECURSIVLY CALL
        let anchor_layer_id = X::anchor_layer_id;
        let anchor_lower = lower.align_to_layer_id(anchor_layer_id);
        let anchor_upper = upper.align_to_layer_id(anchor_layer_id);
        let lower_xyz = anchor_lower.get();
        let upper_xyz = anchor_upper.get();
        let w = index::two_pow_u(level) as u32;
        let mut x = lower_xyz[0];
        while x <= upper_xyz[0] {
            let mut y = lower_xyz[1];
            while y <= upper_xyz[1] {
                let mut z = lower_xyz[2];
                while z <= upper_xyz[2] {
                    let xyz = LodIndex::new(Vec3::new(x,y,z));
                    let i = self.anchor[&xyz];
                    X::drill_down(self, anchor_layer_id, i);
                    z += w;
                }
                y += w;
            }
            x += w;
        }
    }
    fn make_at_most(&mut self, lower: LodIndex, upper: LodIndex, level: i8) {

    }
    fn make_exactly(&mut self, lower: LodIndex, upper: LodIndex, level: i8) {

    }
}

impl LayerInfo for () {
    fn get_child_index(self: &Self) -> Option<usize> {
        None
    }
    const child_layer_id: Option<u8> = Some(0);
    const layer_volume: Vec3<u32> = Vec3{x: 1, y: 1,z: 1};
    const child_len: usize = 0;
}

/*
#[derive(Debug, Clone)]
pub struct LodLayer<E> {
    pub data: E,
    pub childs: Vec<LodLayer<E>>, //Optimization potential: size_of<Vec> == 24 and last layer doesnt need Vec at all.
}

pub trait Layer: Sized {
    fn new() -> LodLayer<Self>;

    fn get_level(layer: &LodLayer<Self>) -> i8;
    fn get_lower_level(layer: &LodLayer<Self>) -> Option<i8>;

    /*Drills down the layer and creates childs*/
    fn drill_down(layer: &mut  LodLayer<Self>);

    /*needs to recalc parent values and remove childs*/
    fn drill_up(parent: &mut LodLayer<Self>);
}

impl<E> LodLayer<E> {
    pub fn new_data(data: E) -> Self {
        Self {
            data,
            childs: Vec::new(),
        }
    }
}

impl<E: Layer> LodLayer<E> {
    // gets the internal index on this layer from relative position

    fn get_internal_index(&self, relative: LodIndex) -> Vec3<u16> {
        let ll = length_to_index(E::get_lower_level(self).expect("your type is wrong configured!, configure Layer trait correctly"));
        let length_per_children: u16 = two_pow_u(ll);
        let child_index = relative.map(|i| (i / length_per_children));
        return child_index;
    }

    fn get_internal_index_and_remainder(&self, relative: LodIndex) -> (Vec3<u16>, LodIndex) {
        let ll = length_to_index(E::get_lower_level(self).expect("your type is wrong configured!, configure Layer trait correctly"));
        let length_per_children: u16 = two_pow_u(ll);
        let child_index = relative.map(|i| (i / length_per_children));
        let remainder_index = relative.map2(child_index, |i,c| (i - c * length_per_children));
        return (child_index, remainder_index);
    }

    /*flatten the (1,2,3) child to 1*4+2*4+3*3*4 = 48*/
    fn get_flat_index(&self, internal_index: Vec3<u16>) -> usize {
        let ll = E::get_lower_level(self).expect("your type is wrong configured!, configure Layer trait correctly");
        let cl = E::get_level(self);
        let childs_per_dimentsion = (cl - ll) as usize;
        let index = internal_index.x as usize + internal_index.y as usize * childs_per_dimentsion + internal_index.z as usize * childs_per_dimentsion * childs_per_dimentsion;
        return index;
    }

    //index must be local to self
    fn get(&self, relative: LodIndex) -> &LodLayer<E> {
        // index is local for now
        if self.childs.is_empty() {
            return &self
        } else {
            let (int, rem) = self.get_internal_index_and_remainder(relative);
            let index = self.get_flat_index(int);
            &self.childs.get(index).unwrap().get(rem)
        }
    }


}
*/
