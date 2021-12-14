pub mod house;
pub mod keep;

use super::skeleton::*;
use crate::{site::BlockMask, IndexRef};
use common::calendar::Calendar;
use rand::prelude::*;
use serde::Deserialize;
use vek::*;

#[derive(Deserialize)]
pub struct Colors {
    pub house: house::Colors,
    pub keep: keep::Colors,
}

pub trait Archetype {
    type Attr;

    fn generate<R: Rng>(rng: &mut R, calendar: Option<&Calendar>) -> (Self, Skeleton<Self::Attr>)
    where
        Self: Sized;

    fn draw(
        &self,
        index: IndexRef,
        pos: Vec3<i32>,
        dist: i32,
        bound_offset: Vec2<i32>,
        center_offset: Vec2<i32>,
        z: i32,
        ori: Ori,
        locus: i32,
        len: i32,
        attr: &Self::Attr,
    ) -> BlockMask;
}
