pub use crate::regs::fields::{Control, Status};
pub use fields::*;

#[derive(derive_mmio::Mmio)]
#[repr(C)]
pub struct Registers {
    mm2s_control: Control,
    mm2s_status: Status,
    _gap0: [u32; 0x4],
    mm2s_source_address_lower_word: u32,
    mm2s_source_address_upper_word: u32,
    _gap1: [u32; 0x2],
    mm2s_transfer_length: fields::Mm2SLength,

    _gap2: u32,

    s2mm_control: Control,
    s2mm_status: Status,
    _gap3: [u32; 0x4],
    s2mm_dest_address_lower_word: u32,
    s2mm_dest_address_upper_word: u32,
    _gap4: [u32; 0x2],
    s2mm_length: u32,
}

static_assertions::const_assert_eq!(core::mem::size_of::<Registers>(), 0x5C);

pub mod fields {
    use arbitrary_int::u26;

    #[bitbybit::bitfield(
        u32,
        default = 0,
        debug,
        defmt_bitfields(feature = "defmt"),
        forbid_overlaps
    )]
    pub struct Mm2SLength {
        #[bits(0..=25, rw)]
        length: u26,
    }
}
