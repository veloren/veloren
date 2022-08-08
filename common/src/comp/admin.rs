use clap::arg_enum;
use specs::Component;

arg_enum! {
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    pub enum AdminRole {
        Moderator = 0,
        Admin = 1,
    }
}

#[derive(Clone, Copy)]
pub struct Admin(pub AdminRole);

impl Component for Admin {
    type Storage = specs::VecStorage<Self>;
}
