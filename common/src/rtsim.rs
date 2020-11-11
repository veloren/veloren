// We'd like to not have this file in `common`, but sadly there are
// things in `common` that require it (currently, `ServerEvent`). When
// possible, this should be moved to the `rtsim` module in `server`.

use specs_idvs::IdvStorage;
use specs::Component;

pub type RtSimId = usize;

#[derive(Copy, Clone, Debug)]
pub struct RtSimEntity(pub RtSimId);

impl Component for RtSimEntity {
    type Storage = IdvStorage<Self>;
}
