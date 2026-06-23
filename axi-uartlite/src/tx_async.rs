//! # Asynchronous TX support.
//!
//! This module provides support for asynchronous non-blocking TX transfers.
//!
//! It provides a static number of async wakers to allow a configurable amount of pollable
//! [TxFuture]s. Each UARTLite [Tx] instance which performs asynchronous TX operations needs
//! to be to explicitely assigned a waker when creating an awaitable [TxAsync] structure
//! as well as when calling the [on_interrupt_tx] handler.
//!
//! The maximum number of available wakers is configured via the waker feature flags:
//!
//! - `1-waker`
//! - `2-wakers`
//! - `4-wakers`
//! - `8-wakers`
//! - `16-wakers`
//! - `32-wakers`
use core::{cell::RefCell, convert::Infallible, marker::PhantomData, sync::atomic::AtomicBool};

use critical_section::Mutex;
use embassy_sync::waitqueue::AtomicWaker;
use raw_buffer::RawBufSlice;

use crate::{FIFO_DEPTH, Tx};

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
static UART_TX_WAKERS: [AtomicWaker; NUM_WAKERS] = [const { AtomicWaker::new() }; NUM_WAKERS];
static TX_CONTEXTS: [Mutex<RefCell<TxContext>>; NUM_WAKERS] =
    [const { Mutex::new(RefCell::new(TxContext::new())) }; NUM_WAKERS];
// Completion flag. Kept outside of the context structure as an atomic to avoid
// critical section.
static TX_DONE: [AtomicBool; NUM_WAKERS] = [const { AtomicBool::new(false) }; NUM_WAKERS];

/// Invalid waker index for [NUM_WAKERS].
#[derive(Debug, thiserror::Error)]
#[error("invalid waker slot index: {0}")]
pub struct InvalidWakerIndex(pub usize);

/// This is a generic interrupt handler to handle asynchronous UART TX operations for a given
/// UART peripheral.
///
/// The user has to call this once in the interrupt handler responsible if the interrupt was
/// triggered by the UARTLite using [TxAsync]. The relevant [Tx] handle of the UARTLite and the
/// waker slot used for it must be passed as well. [Tx::steal] can be used to create the required
/// handle.
pub fn on_interrupt_tx(uartlite_tx: &mut Tx, waker_slot: usize) {
    if waker_slot >= NUM_WAKERS {
        return;
    }
    let status = uartlite_tx.regs.read_stat_reg();
    // Interrupt are not even enabled.
    if !status.intr_enabled() {
        return;
    }
    let mut context = critical_section::with(|cs| {
        let context_ref = TX_CONTEXTS[waker_slot].borrow(cs);
        *context_ref.borrow()
    });
    // No transfer active.
    if context.slice.is_null() {
        return;
    }
    let slice_len = context.slice.len().unwrap();
    if (context.progress >= slice_len && status.tx_fifo_empty()) || slice_len == 0 {
        // Write back updated context structure.
        critical_section::with(|cs| {
            let context_ref = TX_CONTEXTS[waker_slot].borrow(cs);
            *context_ref.borrow_mut() = context;
        });
        // Transfer is done.
        TX_DONE[waker_slot].store(true, core::sync::atomic::Ordering::Relaxed);
        UART_TX_WAKERS[waker_slot].wake();
        return;
    }
    // Safety: We documented that the user provided slice must outlive the future, so we convert
    // the raw pointer back to the slice here.
    let slice = unsafe { context.slice.get() }.expect("slice is invalid");
    while context.progress < slice_len {
        if uartlite_tx.regs.read_stat_reg().tx_fifo_full() {
            break;
        }
        // Safety: TX structure is owned by the future which does not write into the the data
        // register, so we can assume we are the only one writing to the data register.
        uartlite_tx.write_fifo_unchecked(slice[context.progress]);
        context.progress += 1;
    }
    // Write back updated context structure.
    critical_section::with(|cs| {
        let context_ref = TX_CONTEXTS[waker_slot].borrow(cs);
        *context_ref.borrow_mut() = context;
    });
}

/// TX context structure.
#[derive(Debug, Copy, Clone)]
pub struct TxContext {
    progress: usize,
    slice: RawBufSlice,
}

#[allow(clippy::new_without_default)]
impl TxContext {
    /// Create a new TX context structure.
    pub const fn new() -> Self {
        Self {
            progress: 0,
            slice: RawBufSlice::new_nulled(),
        }
    }
}

/// TX future structure.
pub struct TxFuture<'tx, 'buf> {
    waker_idx: usize,
    tx: &'tx mut TxAsync,
    phantom: core::marker::PhantomData<&'buf ()>,
}

impl<'tx, 'buf> TxFuture<'tx, 'buf> {
    /// Create a new TX future which can be used for asynchronous TX operations.
    pub fn new(
        tx: &'tx mut TxAsync,
        waker_idx: usize,
        data: &'buf [u8],
    ) -> Result<Self, InvalidWakerIndex> {
        TX_DONE[waker_idx].store(false, core::sync::atomic::Ordering::Relaxed);
        tx.tx.reset_fifo();

        let init_fill_count = core::cmp::min(data.len(), FIFO_DEPTH);
        // We fill the FIFO with initial data.
        for data in data.iter().take(init_fill_count) {
            tx.tx.write_fifo_unchecked(*data);
        }
        critical_section::with(|cs| {
            let context_ref = TX_CONTEXTS[waker_idx].borrow(cs);
            let mut context = context_ref.borrow_mut();
            unsafe {
                context.slice.set(data);
            }
            context.progress = init_fill_count;
        });
        Ok(Self {
            waker_idx,
            tx,
            phantom: PhantomData,
        })
    }
}

impl Future for TxFuture<'_, '_> {
    type Output = usize;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        UART_TX_WAKERS[self.waker_idx].register(cx.waker());
        if TX_DONE[self.waker_idx].swap(false, core::sync::atomic::Ordering::Relaxed) {
            let progress = critical_section::with(|cs| {
                let mut ctx = TX_CONTEXTS[self.waker_idx].borrow(cs).borrow_mut();
                ctx.slice.set_null();
                ctx.progress
            });
            return core::task::Poll::Ready(progress);
        }
        core::task::Poll::Pending
    }
}

impl Drop for TxFuture<'_, '_> {
    fn drop(&mut self) {
        if !TX_DONE[self.waker_idx].load(core::sync::atomic::Ordering::Relaxed) {
            critical_section::with(|cs| {
                let context_ref = TX_CONTEXTS[self.waker_idx].borrow(cs);
                let mut context_mut = context_ref.borrow_mut();
                context_mut.slice.set_null();
                context_mut.progress = 0;
                // We can not disable interrupts, might be active for RX as well.
                self.tx.tx.reset_fifo();
            });
        }
    }
}

/// Asynchronous TX driver.
pub struct TxAsync {
    pub(crate) tx: Tx,
    waker_idx: usize,
}

impl TxAsync {
    /// Create a new asynchronous TX structure.
    ///
    /// # Safety
    ///
    /// The user MUST ensure that the `Drop` method of all futures generated with this driver
    /// is called on transfer cancellation. By default, this does not require any special handling.
    pub unsafe fn new(tx: Tx, waker_idx: usize) -> Result<Self, InvalidWakerIndex> {
        if waker_idx >= NUM_WAKERS {
            return Err(InvalidWakerIndex(waker_idx));
        }
        Ok(Self { tx, waker_idx })
    }

    /// Write a buffer asynchronously.
    ///
    /// This implementation is not side effect free, and a started future might have already
    /// written part of the passed buffer.
    pub fn write<'buf>(&mut self, buf: &'buf [u8]) -> TxFuture<'_, 'buf> {
        TxFuture::new(self, self.waker_idx, buf).expect("waker index unexpectedly invalid")
    }

    /// Release the owned TX structure.
    pub fn release(self) -> Tx {
        self.tx
    }
}

impl embedded_io::ErrorType for TxAsync {
    type Error = Infallible;
}

impl embedded_io_async::Write for TxAsync {
    /// Write a buffer asynchronously.
    ///
    /// This implementation is not side effect free, and a started future might have already
    /// written part of the passed buffer.
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        Ok(self.write(buf).await)
    }

    /// This implementation does not do anything.
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
