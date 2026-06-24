use core::{cell::UnsafeCell, mem::MaybeUninit};

use vcell::VolatileCell;

pub use crate::regs::fields::{Control, Status};

#[derive(derive_mmio::Mmio)]
#[repr(C)]
pub struct Registers {
    mm2s_control: Control,
    mm2s_status: Status,
    /// The lower 6 bits are reserved and ignored for writes. The address must be 16 word
    /// aligned (e.g. 0x40, 0x80).
    mm2s_current_descriptor_pointer_lower_word: u32,
    mm2s_current_descriptor_pointer_upper_word: u32,
    mm2s_tail_descriptor_lower_word: u32,
    mm2s_tail_descriptor_upper_word: u32,

    _gap0: [u32; 0x5],

    scatter_gather_control: u32,
    s2mm_control: Control,
    s2mm_status: Status,
    /// The lower 6 bits are reserved and ignored for writes. The address must be 16 word
    /// aligned (e.g. 0x40, 0x80).
    s2mm_current_descriptor_pointer_lower_word: u32,
    s2mm_current_descriptor_pointer_upper_word: u32,
    s2mm_tail_descriptor_lower_word: u32,
    s2mm_tail_descriptor_upper_word: u32,
}

static_assertions::const_assert_eq!(core::mem::size_of::<Registers>(), 0x48);

pub mod fields {
    use arbitrary_int::{u4, u26};

    #[bitbybit::bitfield(
        u32,
        default = 0,
        debug,
        defmt_bitfields(feature = "defmt"),
        forbid_overlaps
    )]
    pub struct SgControl {
        #[bits(8..=11, rw)]
        user: u4,
        #[bits(0..=3, rw)]
        cache: u4,
    }

    #[bitbybit::bitfield(
        u32,
        default = 0,
        debug,
        defmt_bitfields(feature = "defmt"),
        forbid_overlaps
    )]
    pub struct DescriptorControl {
        /// Should be set by the CPU to indicate that this descriptor describes the start of the
        /// packet.
        #[bit(27, rw)]
        tx_start_of_frame: bool,
        /// Should be set by the CPU to indicate that this descriptor describes the end of the
        /// packet.
        #[bit(26, rw)]
        tx_end_of_frame: bool,
        #[bits(0..=25, rw)]
        buffer_length: u26,
    }

    #[bitbybit::bitfield(
        u32,
        default = 0,
        debug,
        defmt_bitfields(feature = "defmt"),
        forbid_overlaps
    )]
    pub struct DescriptorStatus {
        #[bit(31, rw)]
        completed: bool,
        #[bit(30, rw)]
        dma_decode_error: bool,
        #[bit(29, rw)]
        dma_slave_error: bool,
        #[bit(28, rw)]
        dma_internal_error: bool,
        #[bits(0..=25, rw)]
        transferred_bytes: u26,
    }
}

#[repr(C, align(0x40))]
pub struct Descriptor {
    /// The lower 6 bits are reserved and ignored for writes. The address must be 16 word
    /// aligned (e.g. 0x40, 0x80).
    next_descriptor_pointer_lower_word: VolatileCell<u32>,
    next_descriptor_pointer_upper_word: VolatileCell<u32>,
    buffer_address_lower_word: VolatileCell<u32>,
    buffer_address_upper_word: VolatileCell<u32>,
    _reserved: [VolatileCell<u32>; 2],
    control: VolatileCell<fields::DescriptorControl>,
    status: VolatileCell<fields::DescriptorStatus>,
    app_words: [VolatileCell<u32>; 5],
}

impl Descriptor {
    #[inline]
    pub const fn new() -> Self {
        Self {
            next_descriptor_pointer_lower_word: VolatileCell::new(0),
            next_descriptor_pointer_upper_word: VolatileCell::new(0),
            buffer_address_lower_word: VolatileCell::new(0),
            buffer_address_upper_word: VolatileCell::new(0),
            _reserved: [const { VolatileCell::new(0) }; 2],
            control: VolatileCell::new(fields::DescriptorControl::new_with_raw_value(0)),
            status: VolatileCell::new(fields::DescriptorStatus::new_with_raw_value(0)),
            app_words: [const { VolatileCell::new(0) }; 5],
        }
    }

    #[inline]
    pub fn status_word(&mut self) -> fields::DescriptorStatus {
        self.status.get()
    }
}

impl Default for Descriptor {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// This is a low level wrapper to simplify declaring a global descriptor list.
///
/// It allows placing the descriptor structure statically in memory which might not
/// be zero-initialized.
#[repr(transparent)]
pub struct DescriptorList<const SLOTS: usize>(pub UnsafeCell<MaybeUninit<[Descriptor; SLOTS]>>);

unsafe impl<const SLOTS: usize> Sync for DescriptorList<SLOTS> {}

impl<const SLOTS: usize> DescriptorList<SLOTS> {
    #[inline]
    pub const fn new() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    /// Initializes the RX descriptors and returns a mutable reference to them.
    ///
    /// # Safety
    ///
    /// This allows creating aliasing mutable references and circumventing ownership and safety
    /// guarantees of the HAL. You MUST call this function only once per descriptor instance.
    pub unsafe fn take(&self) -> &'static mut [Descriptor; SLOTS] {
        let descr = unsafe { &mut *self.0.get() };
        descr.write([const { Descriptor::new() }; SLOTS]);
        unsafe { descr.assume_init_mut() }
    }
}

impl<const SLOTS: usize> Default for DescriptorList<SLOTS> {
    fn default() -> Self {
        Self::new()
    }
}

/// Configures a descriptor list cyclic by setting the pointer to the next descriptor of the
/// last descriptor to the first descriptor.
pub fn configure_descriptors_cyclic(descriptors: &mut [Descriptor]) {
    if descriptors.is_empty() {
        return;
    }
    let addr_of_first_descriptor = descriptors.as_ptr() as usize;
    let last_descriptor = descriptors.last_mut().unwrap();
    if core::mem::size_of::<usize>() == 4 {
        last_descriptor
            .next_descriptor_pointer_lower_word
            .set(addr_of_first_descriptor as u32);
    } else if core::mem::size_of::<usize>() == 8 {
        last_descriptor
            .next_descriptor_pointer_lower_word
            .set(addr_of_first_descriptor as u32);
        last_descriptor
            .next_descriptor_pointer_upper_word
            .set((addr_of_first_descriptor >> 32) as u32);
    }
}
