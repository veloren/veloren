extern crate worldgen;

use worldgen::MacroWorld;

pub fn simulate(mw: &mut MacroWorld, dt: f64) {
    mw.tick(dt);
    mw.calc_wind();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
