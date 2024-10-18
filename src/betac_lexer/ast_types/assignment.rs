use crate::betac_packer::pack::Vis;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssignmentMeta(u8);

impl AssignmentMeta {
    pub const STATIC: u8 = 1 << 0;
    pub const CONSTEXPR: u8 = 1 << 1;
    pub const MUTABLE: u8 = 1 << 2;
    pub const PUBLIC: u8 = 1 << 3;

    pub(crate) fn new() -> Self {
        Self(0)
    }

    pub fn add(mut self, flag: u8) -> Self {
        self.0 |= flag;
        self
    }

    pub const fn is_static(&self) -> bool {
        self.0 & Self::STATIC != 0
    }

    pub const fn is_constexpr(&self) -> bool {
        self.0 & Self::CONSTEXPR != 0
    }

    pub const fn is_mutable(&self) -> bool {
        self.0 & Self::MUTABLE != 0
    }

    pub const fn is_public(&self) -> bool {
        self.0 & Self::PUBLIC != 0
    }

    pub const fn to_vis(&self) -> Vis {
        if self.is_public() {
            Vis::Public
        } else {
            Vis::Private
        }
    }
}
