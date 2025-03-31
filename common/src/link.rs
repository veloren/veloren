use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, SystemData};
use std::{
    any::Any,
    ops::Deref,
    sync::{Arc, Weak},
};

pub trait Link: Sized + Send + Sync + 'static {
    type Error;

    type CreateData<'a>: SystemData<'a>;
    fn create(this: &LinkHandle<Self>, data: &mut Self::CreateData<'_>) -> Result<(), Self::Error>;

    type PersistData<'a>: SystemData<'a>;
    fn persist(this: &LinkHandle<Self>, data: &mut Self::PersistData<'_>) -> bool;

    type DeleteData<'a>: SystemData<'a>;
    fn delete(this: &LinkHandle<Self>, data: &mut Self::DeleteData<'_>);
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
    pub fn delete(&self, data: &mut <R::Link as Link>::DeleteData<'_>) {
        Link::delete(&self.link, data)
    }

    pub fn get_link(&self) -> &LinkHandle<R::Link> { &self.link }
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

    pub fn downgrade(&self) -> WeakLinkHandle<L> {
        WeakLinkHandle {
            link: Arc::downgrade(&self.link),
        }
    }
}

impl<L: Link> Deref for LinkHandle<L> {
    type Target = L;

    fn deref(&self) -> &Self::Target { &self.link }
}

/// The inner data is not shared with the client, so it can't rely on this.
#[derive(Serialize, Deserialize, Debug)]
pub struct WeakLinkHandle<L: Link> {
    #[serde(skip)]
    link: Weak<L>,
}

impl<L: Link> Clone for WeakLinkHandle<L> {
    fn clone(&self) -> Self {
        Self {
            link: Weak::clone(&self.link),
        }
    }
}

impl<L: Link> WeakLinkHandle<L> {
    pub fn upgrade(&self) -> Option<LinkHandle<L>> {
        Some(LinkHandle {
            link: self.link.upgrade()?,
        })
    }

    pub fn into_dyn(self) -> DynWeakLinkHandle {
        DynWeakLinkHandle {
            inner: InnerDynWeakLinkHandle(self.link as DynWeak),
        }
    }
}

type DynWeak = Weak<dyn Any + Sync + Send>;

#[derive(Clone)]
struct InnerDynWeakLinkHandle(DynWeak);

impl Default for InnerDynWeakLinkHandle {
    fn default() -> Self { Self(Weak::<()>::new() as DynWeak) }
}

impl PartialEq for InnerDynWeakLinkHandle {
    fn eq(&self, other: &Self) -> bool { self.0.ptr_eq(&other.0) }
}

impl Eq for InnerDynWeakLinkHandle {}

/// The inner data is not shared with the client, so it can't rely on this.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DynWeakLinkHandle {
    #[serde(skip)]
    inner: InnerDynWeakLinkHandle,
}

impl std::fmt::Debug for DynWeakLinkHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DynWeakLinkHandle")
            .field(&self.exists())
            .finish()
    }
}

impl DynWeakLinkHandle {
    /// If the link this refers to still exists.
    pub fn exists(&self) -> bool { self.inner.0.strong_count() > 0 }

    /// If this is the same link as `link`.
    pub fn is_link(&self, link: &LinkHandle<impl Link>) -> bool {
        std::ptr::addr_eq(self.inner.0.as_ptr(), Arc::as_ptr(&link.link))
    }
}
