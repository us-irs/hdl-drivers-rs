pub mod direct_register;
pub mod scatter_gather;

pub mod fields {
    #[bitbybit::bitenum(u1, exhaustive = true)]
    #[derive(Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "defmt", derive(defmt::Format))]
    pub enum RunStop {
        Stop = 0,
        Run = 1,
    }

    #[bitbybit::bitfield(
        u32,
        default = 0,
        debug,
        defmt_bitfields(feature = "defmt"),
        forbid_overlaps
    )]
    pub struct Control {
        #[bits(24..=31, rw)]
        interrupt_delay: u8,
        #[bits(16..=23, rw)]
        interrupt_threshold: u8,
        #[bit(14, rw)]
        error_interrupt_enable: bool,
        #[bit(13, rw)]
        delay_timer_interrupt_enable: bool,
        #[bit(12, rw)]
        interrupt_on_complete: bool,
        #[bit(4, rw)]
        cyclic_bd_enable: bool,
        #[bit(3, rw)]
        keyhole: bool,
        #[bit(2, rw)]
        reset: bool,
        #[bit(0, rw)]
        run_stop: RunStop,
    }

    #[bitbybit::bitfield(
        u32,
        default = 0,
        debug,
        defmt_bitfields(feature = "defmt"),
        forbid_overlaps
    )]
    pub struct Status {
        #[bits(24..=31, r)]
        interrupt_delay: u8,
        #[bits(16..=23, r)]
        interrupt_threshold: u8,
        /// Write-to-clear interrupt status bit.
        #[bit(14, rw)]
        error_interrupt: bool,
        /// Write-to-clear interrupt status bit.
        #[bit(13, rw)]
        delay_timer_interrupt: bool,
        /// Write-to-clear interrupt status bit.
        #[bit(12, rw)]
        completion_interrupt: bool,
        #[bit(10, r)]
        sg_decode_error: bool,
        #[bit(9, r)]
        sg_slave_error: bool,
        #[bit(8, r)]
        sg_internal_error: bool,
        #[bit(6, r)]
        dma_decode_error: bool,
        #[bit(5, r)]
        dma_slave_error: bool,
        #[bit(4, r)]
        dma_internal_error: bool,
        #[bit(3, r)]
        scatter_gather_enabled: bool,
        #[bit(1, r)]
        idle: bool,
        #[bit(0, r)]
        halted: bool,
    }
}
