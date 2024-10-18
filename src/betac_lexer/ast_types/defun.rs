use crate::{betac_packer::pack::Vis, betac_util::Yarn};

use super::Ty;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefunMeta(u8);

pub type Argument<'a> = (Yarn<'a>, Ty);

impl DefunMeta {
    pub const CONSTEXPR: u8 = 1 << 0;
    pub const UNSAFE: u8 = 1 << 1;
    pub const PUBLIC: u8 = 1 << 2;
    pub const CONSUMER: u8 = 1 << 3;
    pub const MUTABLE: u8 = 1 << 4;

    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn add(mut self, flag: u8) -> Self {
        self.0 |= flag;
        self
    }

    pub const fn is_constexpr(&self) -> bool {
        self.0 & Self::CONSTEXPR != 0
    }

    pub const fn is_unsafe(&self) -> bool {
        self.0 & Self::UNSAFE != 0
    }

    pub const fn is_public(&self) -> bool {
        self.0 & Self::PUBLIC != 0
    }

    pub const fn is_private(&self) -> bool {
        !self.is_public()
    }

    pub const fn is_consumer(&self) -> bool {
        self.0 & Self::CONSUMER != 0
    }

    pub const fn is_mutable(&self) -> bool {
        self.0 & Self::MUTABLE != 0
    }

    pub const fn to_vis(self) -> Vis {
        if self.is_public() {
            Vis::Public
        } else {
            Vis::Private
        }
    }
}
