use std::cmp;
use coord::prelude::*;
use {Volume, Voxel, Cell};

/*

Sphere collision is quite easy, we have a look at the middle and their distance and the added radius of those
This way we can also test Sheres with an offset
    _     _
   / \   / \
  | X \ | X \
   \ /   \ /
   ``    ``

*/

pub trait SphereColission {
    fn middle(&self) -> Vec3<f64>;
    fn radius(&self) -> f64;
}

struct Sphere {
    middle: Vec3<f64>,
    radius: f64,
}

impl SphereColission for Sphere {
    fn middle(&self) -> Vec3<f64> {
        return self.middle;
    }
    fn radius(&self) -> f64 {
        return self.radius;
    }
}

pub fn do_colide_sphere<V: SphereColission, W: SphereColission>(e1: V, e2: W) -> bool {
    //fast check via sphere collision
    let m1 = e1.middle();
    let m2 = e2.middle();
    let r1 = e1.radius();
    let r2 = e2.radius();
    let mdiff = m2 - m1;
    let radd = r1  + r2;
    // compare the squares instead of calulate the squareroot of mlen
    let radd = radd * radd;
    let mlen = mdiff.x*mdiff.x + mdiff.y*mdiff.y + mdiff.z*mdiff.z;
    return radd >= mlen;
}

/*

 CuboidColission 2D representation of 3D cuboid

 +-----+ +-----+
 |     | |     |
 |  X  | |  X  |
 |     | |     |
 +-----+ +-----+

 +-----+-----+
 |     |     |
 |  X  |  X  |
 |     |     |
 +-----+-----+

  /\ +-----+
 /  \|     |
/    \     |
\  X  \ X  |
 \   /     |
  \ /|     |
   - +-----+

Cuboid Colission is quite difficult, because they have a rotation. Our approach will be:
    - take all 8 edges of cuboid1
    - calculate distance to cuboid2 middle
    - check the thickness of cuboid2 towards this point( see below X marks the point, and ``_O marks the thickness)
    - if thickness is bigger or equals to distance from middle2 to point we are good.

          +-----X
+------+   |     |
|     _O   |  X  |
|  X`` |   |     |
|      |   +-----+
+------+

Flaws:
    - actually there is a flaw if objects do not collide in their edges, but this is extremly uncommon, e.g:alloc

  +-+
  | |
+--------------------------------+
| | |           X                |
+--------------------------------+
  | |
  |X|
  | |
  | |
  | |
  +-+

*/

pub trait CuboidColission {
    fn middle(&self) -> Vec3<f64>;
    fn radius(&self) -> Vec3<f64>;
    fn rotation(&self) -> Vec3<f64>;
}

impl<V: Volume> CuboidColission for V {
    fn middle(&self) -> Vec3<f64> {
        let o = self.offset();
        let s = self.size();
        let o = Vec3::new(o.x as f64, o.y as f64, o.z as f64);
        let s = Vec3::new(s.x as f64, s.y as f64, s.z as f64);
        let bl = o + s / 2.0;
        return bl * self.scale();
    }
    // radius of a inner Sphere, or a/2, outer radius would be radius * SQRT(3)
    fn radius(&self) -> Vec3<f64> {
        let s = self.size();
        let s = Vec3::new(s.x as f64 / 2.0, s.y as f64 / 2.0, s.z as f64 / 2.0);
        return s * self.scale();
    }
    fn rotation(&self) -> Vec3<f64> {
        return self.rotation();
    }
}
pub fn might_colide<V: CuboidColission, W: CuboidColission>(e1: V, e2: W) -> bool {
    //fast check via sphere collision
    const SQRT3 : f64 = 1.73205080758;
    let r1 = e1.radius();
    let r2 = e2.radius();
    let biggest_r1 = r1.x.max(r1.y.max(r1.z)) * SQRT3;
    let biggest_r2 = r2.x.max(r2.y.max(r2.z)) * SQRT3;
    let s1 = Sphere{middle: e1.middle(), radius: biggest_r1};
    let s2 = Sphere{middle: e2.middle(), radius: biggest_r2};
    return do_colide_sphere(s1,s2);
}

pub fn do_colide<V: CuboidColission, W: CuboidColission>(e1: V, e2: W) -> bool {
    //do what i told above
    panic!("not implemented");
}
