use crate::regs;

pub struct Adc {
    mmio: regs::adc::MmioRegisters<'static>,
}

impl Adc {
    /// Create a new ADC driver.
    ///
    /// You have to provide the base address of the IP core to the constructor. This needs to be
    /// the base address of the IP core without the ADC block offset.
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
    pub fn new(base_addr_ip_core: usize) -> Self {
        let mmio = regs::Registers::new_adc_block(base_addr_ip_core);
        Adc { mmio }
    }

    pub fn regs(&mut self) -> &mut regs::adc::MmioRegisters<'static> {
        &mut self.mmio
    }
}
