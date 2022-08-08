use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, SystemData};
use std::{ops::Deref, sync::Arc};

pub trait Link: Sized + Send + Sync + 'static {
    type Error;

    type CreateData<'a>: SystemData<'a>;
    fn create(this: &LinkHandle<Self>, data: Self::CreateData<'_>) -> Result<(), Self::Error>;

    type PersistData<'a>: SystemData<'a>;
    fn persist(this: &LinkHandle<Self>, data: Self::PersistData<'_>) -> bool;

    type DeleteData<'a>: SystemData<'a>;
    fn delete(this: &LinkHandle<Self>, data: Self::DeleteData<'_>);
}

pub trait Role {
    type Link: Link;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Is<R: Role> {
    #[serde(bound(serialize = "R::Link: Serialize"))]
    #[serde(bound(deserialize = "R::Link: Deserialize<'de>"))]
    link: LinkHandle<R::Link>,
}

impl<R: Role> Is<R> {
    pub fn delete(&self, data: <R::Link as Link>::DeleteData<'_>) { Link::delete(&self.link, data) }
}

impl<R: Role> Clone for Is<R> {
    fn clone(&self) -> Self {
        Self {
            link: self.link.clone(),
        }
    }
}

impl<R: Role> Deref for Is<R> {
    type Target = R::Link;

    fn deref(&self) -> &Self::Target { &self.link }
}

impl<R: Role + 'static> Component for Is<R>
where
    R::Link: Send + Sync + 'static,
{
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LinkHandle<L: Link> {
    link: Arc<L>,
}

impl<L: Link> Clone for LinkHandle<L> {
    fn clone(&self) -> Self {
        Self {
            link: Arc::clone(&self.link),
        }
    }
}

impl<L: Link> LinkHandle<L> {
    pub fn from_link(link: L) -> Self {
        Self {
            link: Arc::new(link),
        }
    }

    pub fn make_role<R: Role<Link = L>>(&self) -> Is<R> { Is { link: self.clone() } }
}

impl<L: Link> Deref for LinkHandle<L> {
    type Target = L;

    fn deref(&self) -> &Self::Target { &self.link }
}
