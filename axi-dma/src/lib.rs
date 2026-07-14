#![no_std]

use core::future::poll_fn;

use arbitrary_int::{traits::Integer as _, u26};
use embassy_sync::waitqueue::AtomicWaker;
use portable_atomic::AtomicBool;

use crate::regs::{direct_register::Mm2SLength, fields::Control};
pub mod regs;

/// 1 waker (default).
#[cfg(feature = "1-waker")]
pub const NUM_WAKERS: usize = 1;
/// 2 wakers
#[cfg(feature = "2-wakers")]
pub const NUM_WAKERS: usize = 2;
/// 4 wakers
#[cfg(feature = "4-wakers")]
pub const NUM_WAKERS: usize = 4;
/// 8 wakers
#[cfg(feature = "8-wakers")]
pub const NUM_WAKERS: usize = 8;
/// 16 wakers
#[cfg(feature = "16-wakers")]
pub const NUM_WAKERS: usize = 16;
/// 32 wakers
#[cfg(feature = "32-wakers")]
pub const NUM_WAKERS: usize = 32;

static WAKERS: [AtomicWaker; NUM_WAKERS] = [const { AtomicWaker::new() }; NUM_WAKERS];
static SIMPLE_TRANSFER_DONE: [AtomicBool; NUM_WAKERS] =
    [const { AtomicBool::new(false) }; NUM_WAKERS];

pub struct DmaController {
    regs: regs::direct_register::MmioRegisters<'static>,
}

#[derive(Debug, thiserror::Error)]
#[error(
    "Invalid buffer length for DMA transfer. The length must be less than or equal to 2^26 - 1 bytes."
)]
pub struct InvalidBufferLengthError;

#[derive(Debug, thiserror::Error)]
#[error("invalid waker slot index: {0}")]
pub struct InvalidWakerIndex(pub usize);

impl DmaController {
    /// Create a new AXI DMA controller peripheral driver.
    ///
    /// # Safety
    ///
    /// - The `base_addr` must be a valid memory-mapped register address of an AXI DMA peripheral.
    /// - Dereferencing an invalid or misaligned address results in **undefined behavior**.
    /// - The caller must ensure that no other code concurrently modifies the same peripheral registers
    ///   in an unsynchronized manner to prevent data races.
    /// - This function does not enforce uniqueness of driver instances. Creating multiple instances
    ///   with the same `base_addr` can lead to unintended behavior if not externally synchronized.
    /// - The driver performs **volatile** reads and writes to the provided address.
    pub fn new(base_addr: usize) -> Self {
        let mut regs = unsafe { regs::direct_register::Registers::new_mmio_at(base_addr) };
        regs.write_mm2s_control(Control::ZERO.with_reset(true));
        // TODO: Reset timeout error.
        while regs.read_mm2s_control().reset() {}
        Self { regs }
    }

    /// Create an owned blocking writer for MM2S simple transfers.
    pub fn into_simple_writer(self) -> SimpleDmaWriter {
        SimpleDmaWriter { regs: self.regs }
    }

    /// Create an owned async writer for MM2S simple transfers.
    pub fn into_simple_writer_async(
        self,
        waker_index: usize,
    ) -> Result<SimpleDmaWriterAsync, InvalidWakerIndex> {
        if waker_index >= NUM_WAKERS {
            return Err(InvalidWakerIndex(waker_index));
        }
        Ok(SimpleDmaWriterAsync {
            regs: self.regs,
            waker_index,
        })
    }
}

pub struct SimpleDmaWriter {
    regs: regs::direct_register::MmioRegisters<'static>,
}

impl SimpleDmaWriter {
    /// Blocking write function using DMA.
    ///
    /// Pleaes note that the source address must be aligned to the MM2S memory map data width
    /// if the data realignment engine is not included.
    pub fn write(&mut self, buf: &[u8]) -> Result<(), InvalidBufferLengthError> {
        if buf.len() > u26::MAX.as_usize() {
            return Err(InvalidBufferLengthError);
        }
        write(&mut self.regs, buf)?;
        while !self.regs.read_mm2s_status().idle() {}
        Ok(())
    }
}

impl embedded_io_async::ErrorType for SimpleDmaWriter {
    type Error = InvalidBufferLengthError;
}

impl embedded_io::Write for SimpleDmaWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.write(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub struct SimpleDmaWriterAsync {
    regs: regs::direct_register::MmioRegisters<'static>,
    waker_index: usize,
}

impl core::fmt::Debug for SimpleDmaWriterAsync {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SimpleDmaWriterAsync")
            .field("waker_index", &self.waker_index)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimpleDmaTransferToken {
    base_addr: usize,
    waker_index: usize,
}

impl SimpleDmaTransferToken {
    pub fn base_addr(&self) -> usize {
        self.base_addr
    }

    pub fn waker_index(&self) -> usize {
        self.waker_index
    }
}

impl SimpleDmaWriterAsync {
    pub async fn write(
        &mut self,
        buf: &[u8],
    ) -> Result<SimpleDmaTransferToken, InvalidBufferLengthError> {
        let token = SimpleDmaTransferToken {
            // SAFETY: Only converted to primitive address
            base_addr: unsafe { self.regs.ptr() } as usize,
            waker_index: self.waker_index,
        };
        SIMPLE_TRANSFER_DONE[token.waker_index].store(false, portable_atomic::Ordering::Relaxed);
        write(&mut self.regs, buf)?;
        poll_fn(move |cx| {
            WAKERS[token.waker_index].register(cx.waker());

            if SIMPLE_TRANSFER_DONE[token.waker_index].load(portable_atomic::Ordering::Relaxed) {
                return core::task::Poll::Ready(());
            }
            core::task::Poll::Pending
        })
        .await;
        Ok(token)
    }

    pub unsafe fn on_interrupt(token: &SimpleDmaTransferToken) {
        unsafe { Self::on_interrut_unchecked(token.waker_index, token.base_addr) };
    }

    pub unsafe fn on_interrut_unchecked(waker_index: usize, base_addr: usize) {
        let mut regs = unsafe { regs::direct_register::Registers::new_mmio_at(base_addr) };
        if regs.read_mm2s_status().completion_interrupt() {
            SIMPLE_TRANSFER_DONE[waker_index].store(true, portable_atomic::Ordering::Relaxed);
            WAKERS[waker_index].wake();
            regs.write_mm2s_status(regs::fields::Status::ZERO.with_completion_interrupt(true));
        }
    }
}

impl embedded_io_async::Error for InvalidBufferLengthError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

impl embedded_io_async::ErrorType for SimpleDmaWriterAsync {
    type Error = InvalidBufferLengthError;
}

impl embedded_io_async::Write for SimpleDmaWriterAsync {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.write(buf).await?;
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Blocking write function using DMA.
///
/// Pleaes note that the source address must be aligned to the MM2S memory map data width
/// if the data realignment engine is not included.
fn write(
    regs: &mut regs::direct_register::MmioRegisters<'static>,
    buf: &[u8],
) -> Result<(), InvalidBufferLengthError> {
    if buf.len() > u26::MAX.as_usize() {
        return Err(InvalidBufferLengthError);
    }
    regs.modify_mm2s_control(|val| val.with_run_stop(regs::fields::RunStop::Run));
    regs.write_mm2s_source_address_lower_word(buf.as_ptr() as u32);
    regs.write_mm2s_transfer_length(Mm2SLength::ZERO.with_length(u26::new(buf.len() as u32)));
    Ok(())
}

#[cfg(test)]
mod tests {}
