use common::terrain::Block;

#[derive(Copy, Clone)]
pub struct BlockMask {
    block: Option<Block>,
    priority: i32,
}

impl BlockMask {
    pub const fn new(block: Block, priority: i32) -> Self {
        Self {
            block: Some(block),
            priority,
        }
    }

    pub const fn nothing() -> Self {
        Self {
            block: None,
            priority: 0,
        }
    }

    #[must_use]
    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub const fn resolve_with(self, other: Self) -> Self {
        if self.priority >= other.priority {
            self
        } else {
            other
        }
    }

    pub const fn finish(self) -> Option<Block> { self.block }
}
