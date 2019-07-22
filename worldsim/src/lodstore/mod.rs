pub mod index;
use std::u32;
use std::sync::Arc;
use std::collections::HashMap;
use vek::*;
use index::{
    LodIndex,
    AbsIndex,
    relative_to_1d,
};

/*
LOD Data contains different Entries in different vecs, every entry has a "pointer" to it's child start.
This is the structure to store a region and all subscribed information
*/

pub trait LodIntoOptionUsize: Copy {
    fn is_some(self) -> bool;
    fn into_usize(self) -> usize;
}

pub trait LodConfig {
    type L0; // 2^-4
    type L1;
    type L2;
    type L3;
    type L4; // 2^0
    type L5;
    type L6;
    type L7;
    type L8;
    type L9;
    type L10;
    type L11;
    type L12;
    type L13;
    type L14;
    type L15; // 2^11

    type I0: LodIntoOptionUsize;
    type I1: LodIntoOptionUsize;
    type I2: LodIntoOptionUsize;
    type I3: LodIntoOptionUsize;
    type I4: LodIntoOptionUsize;
    type I5: LodIntoOptionUsize;
    type I6: LodIntoOptionUsize;
    type I7: LodIntoOptionUsize;
    type I8: LodIntoOptionUsize;
    type I9: LodIntoOptionUsize;
    type I10: LodIntoOptionUsize;
    type I11: LodIntoOptionUsize;
    type I12: LodIntoOptionUsize;
    type I13: LodIntoOptionUsize;
    type I14: LodIntoOptionUsize;
    type I15: LodIntoOptionUsize;

    const anchor_layer_id: u8;

    const layer_volume: [Vec3<u32>; 16]; // number of elements on this layer as Vec3 (not on child layer!)
    const child_layer_id: [Option<u8>; 16]; // layer below this one
    const layer_len: [usize; 16]; // optimisation for layer_volume, total no of elements as usize

    fn setup(&mut self);
    fn drill_down(data: &mut LodData::<Self>, abs: AbsIndex) where Self: Sized;
    fn drill_up(data: &mut LodData::<Self>, parent_abs: AbsIndex) where Self: Sized;
}

/*
There is another optimization problem: We have OWNED data and foreign DATA in the struct, but we don't want the foreign data to take a lot of space if unused
But both needs to be accessible transparent without overhead in calculation, difficult.
Imagine a Terrain, which top level is L13, so it would have 64 entries for the owned and 1664 for foreign data if everything is filled.
So we really only fill the boarder giving us 152 border areas.
One could think about multiple entry levels for foreign and owned data, but that means, that foreign data without a parent would exist, which might break algorithms....
So for now we go with a single anchorlevel for now, and hope the designer chooses good levels
*/

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

    pub child0: Vec<X::I0>,
    pub child1: Vec<X::I1>,
    pub child2: Vec<X::I2>,
    pub child3: Vec<X::I3>,
    pub child4: Vec<X::I4>,
    pub child5: Vec<X::I5>,
    pub child6: Vec<X::I6>,
    pub child7: Vec<X::I7>,
    pub child8: Vec<X::I8>,
    pub child9: Vec<X::I9>,
    pub child10: Vec<X::I10>,
    pub child11: Vec<X::I11>,
    pub child12: Vec<X::I12>,
    pub child13: Vec<X::I13>,
    pub child14: Vec<X::I14>,
    pub child15: Vec<X::I15>,
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
            child0: Vec::new(),
            child1: Vec::new(),
            child2: Vec::new(),
            child3: Vec::new(),
            child4: Vec::new(),
            child5: Vec::new(),
            child6: Vec::new(),
            child7: Vec::new(),
            child8: Vec::new(),
            child9: Vec::new(),
            child10: Vec::new(),
            child11: Vec::new(),
            child12: Vec::new(),
            child13: Vec::new(),
            child14: Vec::new(),
            child15: Vec::new(),
            anchor: HashMap::new(),
        }
    }

    // dynamically dispatches the get_child_index, this is most prob the bottleneck function.
    // evaluate the performacne impact!!!
    fn int_get_child_index(&self, abs: AbsIndex) -> usize {
        match abs.layer {
            0 => self.child0[abs.index].into_usize(),
            1 => self.child1[abs.index].into_usize(),
            2 => self.child2[abs.index].into_usize(),
            3 => self.child3[abs.index].into_usize(),
            4 => self.child4[abs.index].into_usize(),
            5 => self.child5[abs.index].into_usize(),
            6 => self.child6[abs.index].into_usize(),
            7 => self.child7[abs.index].into_usize(),
            8 => self.child8[abs.index].into_usize(),
            9 => self.child9[abs.index].into_usize(),
            10 => self.child10[abs.index].into_usize(),
            11 => self.child11[abs.index].into_usize(),
            12 => self.child12[abs.index].into_usize(),
            13 => self.child13[abs.index].into_usize(),
            14 => self.child14[abs.index].into_usize(),
            15 => self.child15[abs.index].into_usize(),
            _ => panic!("wrong abs index"),
        }
    }

    // Returns the childs AbsIndex of Parent AbsIndex
    // child_lod must lie within parent
    // uses parent_lod as buffer, to not calculate it again
    // uses parent_child_index as a buffer, to not calculate it again
    fn int_get(parent_abs: AbsIndex, child_lod: LodIndex, parent_lod: LodIndex, parent_child_index: usize) -> AbsIndex {
        let child_layer = X::child_layer_id[parent_abs.layer as usize].unwrap();
        let child_lod = child_lod.align_to_layer_id(child_layer);
        let child_offset = relative_to_1d(child_lod, parent_lod, child_layer, X::layer_volume[child_layer as usize]);
        //println!("{} int_get - parent_abs {} child_lod {} parent_lod {} parent_child_index {} child_offset {}", Self::ppp(parent_abs.layer), parent_abs, child_lod, parent_lod, parent_child_index, child_offset);
        AbsIndex::new(child_layer, parent_child_index + child_offset)
    }

    // slower variant of int_get which requiere self lookups
    fn int_get_lockup(&self, parent_abs: AbsIndex, child_lod: LodIndex) -> AbsIndex {
        let parent_lod = child_lod.align_to_layer_id(parent_abs.layer);
        let parent_child_index = self.int_get_child_index(parent_abs);
        Self::int_get(parent_abs, child_lod, parent_lod, parent_child_index)
    }

    // target_layer is requiered because same LodIndex can exist for multiple layers, and guessing is stupid here
    fn int_recursive_get(&self, parent_abs: AbsIndex, child_lod: LodIndex, target_layer:u8) -> AbsIndex {
        let mut parent_abs = parent_abs;
        while true {
            //println!("{} int_recursive_get {} - {}", Self::ppp(parent_abs.layer), parent_abs, target_layer);
            parent_abs = self.int_get_lockup(parent_abs, child_lod);
            if parent_abs.layer <= target_layer {
                return parent_abs;
            }
        }
        unreachable!();
    }

    pub fn get15(&self, index: LodIndex) -> &X::L15 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 15);
        debug_assert_eq!(wanted_abs.layer, 15);
        &self.layer15[wanted_abs.index]
    }

    pub fn get14(&self, index: LodIndex) -> &X::L14 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 14);
        debug_assert_eq!(wanted_abs.layer, 14);
        &self.layer14[wanted_abs.index]
    }

    pub fn get13(&self, index: LodIndex) -> &X::L13 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 13);
        debug_assert_eq!(wanted_abs.layer, 13);
        &self.layer13[wanted_abs.index]
    }

    pub fn get12(&self, index: LodIndex) -> &X::L12 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 12);
        debug_assert_eq!(wanted_abs.layer, 12);
        &self.layer12[wanted_abs.index]
    }

    pub fn get11(&self, index: LodIndex) -> &X::L11 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 11);
        debug_assert_eq!(wanted_abs.layer, 11);
        &self.layer11[wanted_abs.index]
    }

    pub fn get10(&self, index: LodIndex) -> &X::L10 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 10);
        debug_assert_eq!(wanted_abs.layer, 10);
        &self.layer10[wanted_abs.index]
    }

    pub fn get9(&self, index: LodIndex) -> &X::L9 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 9);
        debug_assert_eq!(wanted_abs.layer, 9);
        &self.layer9[wanted_abs.index]
    }

    pub fn get8(&self, index: LodIndex) -> &X::L8 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 8);
        debug_assert_eq!(wanted_abs.layer, 8);
        &self.layer8[wanted_abs.index]
    }

    pub fn get7(&self, index: LodIndex) -> &X::L7 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 7);
        debug_assert_eq!(wanted_abs.layer, 7);
        &self.layer7[wanted_abs.index]
    }

    pub fn get6(&self, index: LodIndex) -> &X::L6 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 6);
        debug_assert_eq!(wanted_abs.layer, 6);
        &self.layer6[wanted_abs.index]
    }

    pub fn get5(&self, index: LodIndex) -> &X::L5 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 5);
        debug_assert_eq!(wanted_abs.layer, 5);
        &self.layer5[wanted_abs.index]
    }

    pub fn get4(&self, index: LodIndex) -> &X::L4 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        //println!("4aa4 {:?} - {:?}", anchor_lod, anchor_abs );
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 4);
        debug_assert_eq!(wanted_abs.layer, 4);
        &self.layer4[wanted_abs.index]
    }

    pub fn get3(&self, index: LodIndex) -> &X::L3 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 3);
        debug_assert_eq!(wanted_abs.layer, 3);
        &self.layer3[wanted_abs.index]
    }

    pub fn get2(&self, index: LodIndex) -> &X::L2 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 2);
        debug_assert_eq!(wanted_abs.layer, 2);
        &self.layer2[wanted_abs.index]
    }

    pub fn get1(&self, index: LodIndex) -> &X::L1 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 1);
        debug_assert_eq!(wanted_abs.layer, 1);
        &self.layer1[wanted_abs.index]
    }

    pub fn get0(&self, index: LodIndex) -> &X::L0 {
        let anchor_lod = index.align_to_layer_id(X::anchor_layer_id);
        let anchor_abs = AbsIndex::new(X::anchor_layer_id, self.anchor[&anchor_lod]);
        let wanted_abs = self.int_recursive_get(anchor_abs, index, 0);
        debug_assert_eq!(wanted_abs.layer, 0);
        &self.layer0[wanted_abs.index]
    }

    // returns the last LodIndex, that belongs to a parent AbsIndex
    fn get_last_child_lod(parent: LodIndex, parent_level: u8) -> LodIndex {
        let child_width = index::two_pow_u(X::child_layer_id[parent_level as usize].unwrap()) as u32;
        parent + LodIndex::new(X::layer_volume[X::child_layer_id[parent_level as usize].unwrap() as usize].map(|e| (e-1)*child_width))
    }

    fn ppp(level: u8) -> &'static str {
        match level {
            0 => " | ",
            4 => " ---- ",
            5 => " ----- ",
            9 => " ---------- ",
            13 => " ------------- ",
            _ => panic!("aaa"),
        }
    }

    /*
        lower: must always be a LodIndex inside parent
        upper: must always have same parent as lower -> parent
    */
    fn int_make_at_least(&mut self, parent: AbsIndex, /*parent_lod2: LodIndex,*/ lower: LodIndex, upper: LodIndex, target_level: u8) {
        let child_layer = X::child_layer_id[parent.layer as usize];
        let parent_lod_width = index::two_pow_u(parent.layer) as u32;
        let parent_lod = lower.align_to_layer_id(parent.layer);
        //assert_eq!(parent_lod, parent_lod2);
        //println!("{} lower, upper {} {} {} - {:?}", Self::ppp(parent.layer), lower, upper, parent_lod_width, child_layer);
        if parent.layer > target_level {
            // create necessary childs:
            X::drill_down(self, parent);
            //println!("{} DRILLED DOWN", Self::ppp(parent.layer));
            if child_layer.is_some() && child_layer.unwrap() > target_level {
                let child_layer = child_layer.unwrap();
                let child_lod_width = index::two_pow_u(child_layer) as u32;
                //calc childs which needs to be called recusivly, there childs will be the new parents
                let child_lower = lower.align_to_layer_id(child_layer);
                let child_upper = upper.align_to_layer_id(child_layer);
                let child_base_abs_index = self.int_get_child_index(parent);
                let child_volume = X::layer_volume[child_layer as usize];
                // loop over childs and calculate correct lower and
                let lower_xyz = (child_lower.get()-parent_lod.get()).map(|e| e / child_lod_width);
                let upper_xyz = (child_upper.get()-parent_lod.get()).map(|e| e / child_lod_width);
                //println!("{} lxyz {}", Self::ppp(parent.layer), lower_xyz);
                //println!("{} uxyz {}", Self::ppp(parent.layer), upper_xyz);
                //println!("{} child_lod_width {}", Self::ppp(parent.layer), child_lod_width);
                for x in lower_xyz[0]..upper_xyz[0]+1 {
                    for y in lower_xyz[1]..upper_xyz[1]+1 {
                        for z in lower_xyz[2]..upper_xyz[2]+1 {
                            //println!("{} xyz {} {} {}", Self::ppp(parent.layer), x, y, z);
                            //calculate individual abs values, because we now, how they are ordered in the vec
                            let child_abs_index = child_base_abs_index + (x * child_volume[2] * child_volume[1] + y * child_volume[2] + z) as usize;
                            let child_abs = AbsIndex::new(child_layer, child_abs_index);
                            let child_lower = parent_lod + LodIndex::new(Vec3::new(x * child_lod_width, y * child_lod_width, z * child_lod_width));
                            let child_upper = child_lower + LodIndex::new(Vec3::new(child_lod_width-1, child_lod_width-1, child_lod_width-1));

                            let inner_lower = index::max(lower, child_lower);
                            let inner_upper = index::min(upper, child_upper);
                            //println!("{} restrict {} {} to {} {}", Self::ppp(parent.layer), lower, upper, inner_lower, inner_upper);
                            Self::int_make_at_least(self, child_abs, inner_lower, inner_upper, target_level);
                        }
                    }
                }
            }
        }
    }

    /*
    These functions allow you to make the LodLayer provide a certain LOD for the specified area
    */
    /*is at least minimum or maximum*/

    pub fn make_at_least(&mut self, lower: LodIndex, upper: LodIndex, target_level: u8) {
        //ERROR, DOES NOT RECURSIVLY CALL
        let anchor_layer_id = X::anchor_layer_id;
        let anchor_lower = lower.align_to_layer_id(anchor_layer_id);
        let anchor_upper = upper.align_to_layer_id(anchor_layer_id);
        let lower_xyz = anchor_lower.get();
        let upper_xyz = anchor_upper.get();
        let anchor_width = index::two_pow_u(anchor_layer_id) as u32;
        let mut x = lower_xyz[0];
        //println!("{} xxx lower, upper {} {} {}", Self::ppp(anchor_layer_id), lower_xyz, upper_xyz, anchor_width);
        while x <= upper_xyz[0] {
            let mut y = lower_xyz[1];
            while y <= upper_xyz[1] {
                let mut z = lower_xyz[2];
                while z <= upper_xyz[2] {
                    let anchor_lod = LodIndex::new(Vec3::new(x,y,z));
                    let anchor_abs = AbsIndex::new(anchor_layer_id, self.anchor[&anchor_lod]); ;
                    if anchor_abs.layer > target_level {
                        X::drill_down(self, anchor_abs);
                        let child_lod_upper = Self::get_last_child_lod(anchor_lod, anchor_abs.layer);

                        let inner_lower = index::max(lower, anchor_lod);
                        let inner_upper = index::min(upper, child_lod_upper);

                        //println!("{}call child with lower, upper {} {} instead of {} {} ", Self::ppp(anchor_layer_id), inner_lower, inner_upper, anchor_lod, child_lod_upper);
                        self.int_make_at_least(anchor_abs, inner_lower, inner_upper, target_level);
                    }
                    z += anchor_width;
                }
                y += anchor_width;
            }
            x += anchor_width;
        }
    }
    fn make_at_most(&mut self, lower: LodIndex, upper: LodIndex, level: i8) {

    }
    fn make_exactly(&mut self, lower: LodIndex, upper: LodIndex, level: i8) {

    }
}

impl LodIntoOptionUsize for () {
    fn is_some(self) -> bool {
        false
    }
    fn into_usize(self) -> usize {
        unreachable!("dummyUsize")
    }
}

impl LodIntoOptionUsize for u32 {
    fn is_some(self) -> bool {
        self != u32::MAX
    }
    fn into_usize(self) -> usize {
        self as usize
    }
}