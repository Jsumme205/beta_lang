use crate::{betac_packer::pack::Vis, betac_util::Yarn};

use super::{Metadata, Ty};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefunMeta(u8);

pub type Argument<'a> = (Yarn<'a>, Ty);

impl Metadata for DefunMeta {
    fn init() -> Self {
        Self(0)
    }

    fn add_flag(mut self, flag: u8) -> Self {
        self.0 |= flag;
        self
    }

    fn flag_set(&self, flag: u8) -> bool {
        self.0 & flag != 0
    }
}
