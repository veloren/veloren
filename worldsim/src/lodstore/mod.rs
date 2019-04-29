pub mod index;
use std::sync::Arc;
use vek::*;
use index::{
    LodIndex,
    length_to_index,
    two_pow_u,
};

/*
Alternative impl idea:
1) Put LodLayer to a trait to give everyone the power to store it themself.
2) Put childs in up to 8 different VECs, and have a index in LogLayer, pointing to the fist childred in this supervector, and ge the length from E.
all access is only possible if the Owner sturcture is given
*/

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
            childs: vec!(),
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

    /*
    These functions allow you to make the LodLayer provide a certain LOD for the specified area
    */
    /*is at least minimum or maximum*/
    pub fn make_at_least(&mut self, lower: LodIndex, upper: LodIndex, level: i8) {
        if E::get_level(self) <= level {
            //println!("done");
            return;
        }
        let (li, lr) = self.get_internal_index_and_remainder(lower);
        let (ui, ur) = self.get_internal_index_and_remainder(upper);

        if self.childs.is_empty() {
            E::drill_down(self);
            //println!("dd");
        }
        let ll = length_to_index(E::get_lower_level(self).expect("your type is wrong configured!, configure Layer trait correctly"));
        let length_per_children: u16 = two_pow_u(ll);
        //println!("li {} lr {} ui {} ur {} length {}", li, lr, ui, ur, length_per_children);
        if E::get_lower_level(self).unwrap() <= level {
            //println!("done");
            return;
        }

        for z in li.z..ui.z+1 {
            for y in li.y..ui.y+1 {
                for x in li.x..ui.x+1 {
                    //recursive call make_at_least
                    let mut new_lower = LodIndex::new(0,0,0);
                    let mut new_upper = LodIndex::new(length_per_children-1,length_per_children-1,length_per_children-1);
                    if x == li.x { new_lower.x = lr.x; }
                    if y == li.y { new_lower.y = lr.y; }
                    if z == li.z { new_lower.z = lr.z; }
                    if x == ui.x { new_upper.x = ur.x; }
                    if y == ui.y { new_upper.y = ur.y; }
                    if z == ui.z { new_upper.z = ur.z; }
                    let index = self.get_flat_index(LodIndex::new(x,y,z));
                    //println!("lo {} up {} layer {} inr {} in {}, vec {}", new_lower, new_upper, E::get_level(self), LodIndex::new(x,y,z), index, self.childs.len());
                    self.childs[index].make_at_least( new_lower, new_upper, level );

                }
            }
        }
    }
    fn make_at_most(&mut self, lower: LodIndex, upper: LodIndex, level: i8) {

    }
    fn make_exactly(&mut self, lower: LodIndex, upper: LodIndex, level: i8) {

    }
}

