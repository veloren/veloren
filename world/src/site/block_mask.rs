use common::{terrain::Block, vol::Vox};

#[derive(Copy, Clone)]
pub struct BlockMask {
    block: Block,
    priority: i32,
}

impl BlockMask {
    pub fn new(block: Block, priority: i32) -> Self { Self { block, priority } }

    pub fn nothing() -> Self {
        Self {
            block: Block::empty(),
            priority: 0,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn resolve_with(self, other: Self) -> Self {
        if self.priority >= other.priority {
            self
        } else {
            other
        }
    }

    pub fn finish(self) -> Option<Block> {
        if self.priority > 0 {
            Some(self.block)
        } else {
            None
        }
    }
}
