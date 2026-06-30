use core::num::NonZero;

use arbitrary_int::{traits::Integer as _, u4};

use crate::regs;

pub struct Dac {
    mmio: regs::dac::MmioRegisters<'static>,
}

impl Dac {
    /// Create a new DAC driver.
    ///
    /// You have to provide the base address of the IP core to the constructor. This needs to be
    /// the base address of the IP core without the DAC block offset.
    /// This function also enables the driver and synchronizes the channels within the DAC.
    ///
    /// # Safety
    ///
    /// - The `base_addr_ip_core` must be a valid memory-mapped register address of the
    ///   peripheral.
    /// - Dereferencing an invalid or misaligned address results in **undefined behavior**.
    /// - The caller must ensure that no other code concurrently modifies the same peripheral registers
    ///   in an unsynchronized manner to prevent data races.
    /// - This function does not enforce uniqueness of driver instances. Creating multiple instances
    ///   with the same `base_addr` can lead to unintended behavior if not externally synchronized.
    /// - The driver performs **volatile** reads and writes to the provided address.
    pub fn new(base_addr_ip_core: usize, rate_div: NonZero<u8>) -> Self {
        let mmio = regs::Registers::new_dac_block(base_addr_ip_core);
        let mut dac = Dac { mmio };
        dac.enable();
        dac.set_rate_div(rate_div);
        dac.synchronize();
        dac
    }

    pub fn enable(&mut self) {
        self.mmio
            .write_reset(crate::regs::fields::Reset::ZERO.with_clock_disable(false));
        self.mmio.write_reset(
            crate::regs::fields::Reset::builder()
                .with_clock_disable(false)
                .with_mmcm_reset_n(true)
                .with_reset_n(true)
                .build(),
        );
    }

    /// Also synchronizes the DAC channels.
    pub fn set_data_source(&mut self, channel: u4, source: regs::dac::regs::DataSource) {
        self.mmio
            .dac_channels(channel.as_usize())
            .expect("DAC channel retrieval failed unexpectedly")
            .write_control7(
                regs::dac::regs::ChannelControl7::builder()
                    .with_data_source(source)
                    .build(),
            );
        self.synchronize();
    }

    /// Also synchronizes the DAC channels.
    pub fn set_data_source_all_channels_up_to(
        &mut self,
        num_channels: u4,
        source: regs::dac::regs::DataSource,
    ) {
        for i in 0..num_channels.as_usize() {
            self.mmio
                .dac_channels(i)
                .expect("DAC channel retrieval failed unexpectedly")
                .write_control7(
                    regs::dac::regs::ChannelControl7::builder()
                        .with_data_source(source)
                        .build(),
                );
        }
        self.synchronize();
    }

    pub fn set_rate_div(&mut self, rate_div: NonZero<u8>) {
        self.mmio
            .write_rate_control(regs::dac::regs::RateControl::new_with_raw_value(
                rate_div.get() as u32,
            ));
    }

    pub fn synchronize(&mut self) {
        self.mmio
            .write_control1(regs::dac::regs::Control1::ZERO.with_sync(true));
    }

    pub fn regs(&mut self) -> &mut regs::dac::MmioRegisters<'static> {
        &mut self.mmio
    }
}
