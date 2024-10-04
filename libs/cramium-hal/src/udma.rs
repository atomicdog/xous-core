use core::mem::size_of;

use utralib::*;

use crate::SharedCsr;
use crate::ifram::{IframRange, UdmaWidths};

/// UDMA has a structure that Rust hates. The concept of UDMA is to take a bunch of
/// different hardware functions, and access them with a template register pattern.
/// But with small asterisks here and there depending upon the hardware block in question.
///
/// It is essentially polymorphism at the hardware level, but with special cases meant
/// to be patched up with instance-specific peeks and pokes. It's probably possible
/// to create a type system that can safe-ify this kind of structure, but just because
/// something is possible does not mean it's a good idea to do it, nor would it be
/// maintainable and/or ergonomic to use.
///
/// Anyways. Lots of unsafe code here. UDMA: specious concept, made entirely of footguns.

// --------------------------- Global Shared State (!!🤌!!) --------------------------
#[repr(usize)]
enum GlobalReg {
    ClockGate = 0,
    EventIn = 1,
}
impl Into<usize> for GlobalReg {
    fn into(self) -> usize { self as usize }
}
#[repr(u32)]
#[derive(Copy, Clone, num_derive::FromPrimitive)]
pub enum PeriphId {
    Uart0 = 1 << 0,
    Uart1 = 1 << 1,
    Uart2 = 1 << 2,
    Uart3 = 1 << 3,
    Spim0 = 1 << 4,
    Spim1 = 1 << 5,
    Spim2 = 1 << 6,
    Spim3 = 1 << 7,
    I2c0 = 1 << 8,
    I2c1 = 1 << 9,
    I2c2 = 1 << 10,
    I2c3 = 1 << 11,
    Sdio = 1 << 12,
    I2s = 1 << 13,
    Cam = 1 << 14,
    Filter = 1 << 15,
    Scif = 1 << 16,
    Spis0 = 1 << 17,
    Spis1 = 1 << 18,
    Adc = 1 << 19,
}
impl Into<u32> for PeriphId {
    fn into(self) -> u32 { self as u32 }
}

impl From<SpimChannel> for PeriphId {
    fn from(value: SpimChannel) -> Self {
        match value {
            SpimChannel::Channel0 => PeriphId::Spim0,
            SpimChannel::Channel1 => PeriphId::Spim1,
            SpimChannel::Channel2 => PeriphId::Spim2,
            SpimChannel::Channel3 => PeriphId::Spim3,
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum PeriphEventId {
    Uart0 = 0,
    Uart1 = 4,
    Uart2 = 8,
    Uart3 = 12,
    Spim0 = 16,
    Spim1 = 20,
    Spim2 = 24,
    Spim3 = 28,
    I2c0 = 32,
    I2c1 = 36,
    I2c2 = 40,
    I2c3 = 44,
    Sdio = 48,
    I2s = 52,
    Cam = 56,
    Adc = 57, // note exception to ordering here
    Filter = 60,
    Scif = 64,
    Spis0 = 68,
    Spis1 = 72,
}
impl From<PeriphId> for PeriphEventId {
    fn from(id: PeriphId) -> Self {
        match id {
            PeriphId::Uart0 => PeriphEventId::Uart0,
            PeriphId::Uart1 => PeriphEventId::Uart1,
            PeriphId::Uart2 => PeriphEventId::Uart2,
            PeriphId::Uart3 => PeriphEventId::Uart3,
            PeriphId::Spim0 => PeriphEventId::Spim0,
            PeriphId::Spim1 => PeriphEventId::Spim1,
            PeriphId::Spim2 => PeriphEventId::Spim2,
            PeriphId::Spim3 => PeriphEventId::Spim3,
            PeriphId::I2c0 => PeriphEventId::I2c0,
            PeriphId::I2c1 => PeriphEventId::I2c1,
            PeriphId::I2c2 => PeriphEventId::I2c2,
            PeriphId::I2c3 => PeriphEventId::I2c3,
            PeriphId::Sdio => PeriphEventId::Sdio,
            PeriphId::I2s => PeriphEventId::I2s,
            PeriphId::Cam => PeriphEventId::Cam,
            PeriphId::Filter => PeriphEventId::Filter,
            PeriphId::Scif => PeriphEventId::Scif,
            PeriphId::Spis0 => PeriphEventId::Spis0,
            PeriphId::Spis1 => PeriphEventId::Spis1,
            PeriphId::Adc => PeriphEventId::Adc,
        }
    }
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum EventUartOffset {
    Rx = 0,
    Tx = 1,
    RxChar = 2,
    Err = 3,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum EventSpimOffset {
    Rx = 0,
    Tx = 1,
    Cmd = 2,
    Eot = 3,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum EventI2cOffset {
    Rx = 0,
    Tx = 1,
    Cmd = 2,
    Eot = 3,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum EventSdioOffset {
    Rx = 0,
    Tx = 1,
    Eot = 2,
    Err = 3,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum EventI2sOffset {
    Rx = 0,
    Tx = 1,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum EventCamOffset {
    Rx = 0,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum EventAdcOffset {
    Rx = 0,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum EventFilterOffset {
    Eot = 0,
    Active = 1,
}
#[repr(u32)]
#[derive(Copy, Clone)]

pub enum EventScifOffset {
    Rx = 0,
    Tx = 1,
    RxChar = 2,
    Err = 3,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum EventSpisOffset {
    Rx = 0,
    Tx = 1,
    Eot = 2,
}
#[derive(Copy, Clone)]
pub enum PeriphEventType {
    Uart(EventUartOffset),
    Spim(EventSpimOffset),
    I2c(EventI2cOffset),
    Sdio(EventSdioOffset),
    I2s(EventI2sOffset),
    Cam(EventCamOffset),
    Adc(EventAdcOffset),
    Filter(EventFilterOffset),
    Scif(EventScifOffset),
    Spis(EventSpisOffset),
}
impl Into<u32> for PeriphEventType {
    fn into(self) -> u32 {
        match self {
            PeriphEventType::Uart(t) => t as u32,
            PeriphEventType::Spim(t) => t as u32,
            PeriphEventType::I2c(t) => t as u32,
            PeriphEventType::Sdio(t) => t as u32,
            PeriphEventType::I2s(t) => t as u32,
            PeriphEventType::Cam(t) => t as u32,
            PeriphEventType::Adc(t) => t as u32,
            PeriphEventType::Filter(t) => t as u32,
            PeriphEventType::Scif(t) => t as u32,
            PeriphEventType::Spis(t) => t as u32,
        }
    }
}

/// Use a trait that will allow us to share code between both `std` and `no-std` implementations
pub trait UdmaGlobalConfig {
    fn clock(&self, peripheral: PeriphId, enable: bool);
    unsafe fn udma_event_map(
        &self,
        peripheral: PeriphId,
        event_type: PeriphEventType,
        to_channel: EventChannel,
    );
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, num_derive::FromPrimitive)]
pub enum EventChannel {
    Channel0 = 0,
    Channel1 = 8,
    Channel2 = 16,
    Channel3 = 24,
}
pub struct GlobalConfig {
    csr: SharedCsr<u32>,
}
impl GlobalConfig {
    pub fn new(base_addr: *mut u32) -> Self { GlobalConfig { csr: SharedCsr::new(base_addr) } }

    pub fn clock_on(&self, peripheral: PeriphId) {
        // Safety: only safe when used in the context of UDMA registers.
        unsafe {
            self.csr.base().add(GlobalReg::ClockGate.into()).write_volatile(
                self.csr.base().add(GlobalReg::ClockGate.into()).read_volatile() | peripheral as u32,
            );
        }
    }

    pub fn clock_off(&self, peripheral: PeriphId) {
        // Safety: only safe when used in the context of UDMA registers.
        unsafe {
            self.csr.base().add(GlobalReg::ClockGate.into()).write_volatile(
                self.csr.base().add(GlobalReg::ClockGate.into()).read_volatile() & !(peripheral as u32),
            );
        }
    }

    pub fn raw_clock_map(&self) -> u32 {
        // Safety: only safe when used in the context of UDMA registers.
        unsafe { self.csr.base().add(GlobalReg::ClockGate.into()).read_volatile() }
    }

    pub fn is_clock_set(&self, peripheral: PeriphId) -> bool {
        // Safety: only safe when used in the context of UDMA registers.
        unsafe {
            (self.csr.base().add(GlobalReg::ClockGate.into()).read_volatile() & (peripheral as u32)) != 0
        }
    }

    pub fn map_event(&self, peripheral: PeriphId, event_type: PeriphEventType, to_channel: EventChannel) {
        let event_type: u32 = event_type.into();
        let id: u32 = PeriphEventId::from(peripheral) as u32 + event_type;
        // Safety: only safe when used in the context of UDMA registers.
        unsafe {
            self.csr.base().add(GlobalReg::EventIn.into()).write_volatile(
                self.csr.base().add(GlobalReg::EventIn.into()).read_volatile()
                    & !(0xFF << (to_channel as u32))
                    | id << (to_channel as u32),
            )
        }
    }

    /// Same as map_event(), but for cases where the offset is known. This would typically be the case
    /// where a remote function transformed a PeriphEventType into a primitive `u32` and passed
    /// it through an IPC interface.
    pub fn map_event_with_offset(&self, peripheral: PeriphId, event_offset: u32, to_channel: EventChannel) {
        let id: u32 = PeriphEventId::from(peripheral) as u32 + event_offset;
        // Safety: only safe when used in the context of UDMA registers.
        unsafe {
            self.csr.base().add(GlobalReg::EventIn.into()).write_volatile(
                self.csr.base().add(GlobalReg::EventIn.into()).read_volatile()
                    & !(0xFF << (to_channel as u32))
                    | id << (to_channel as u32),
            )
        }
    }

    pub fn raw_event_map(&self) -> u32 {
        // Safety: only safe when used in the context of UDMA registers.
        unsafe { self.csr.base().add(GlobalReg::EventIn.into()).read_volatile() }
    }
}

impl UdmaGlobalConfig for GlobalConfig {
    fn clock(&self, peripheral: PeriphId, enable: bool) {
        if enable {
            self.clock_on(peripheral);
        } else {
            self.clock_off(peripheral);
        }
    }

    unsafe fn udma_event_map(
        &self,
        peripheral: PeriphId,
        event_type: PeriphEventType,
        to_channel: EventChannel,
    ) {
        self.map_event(peripheral, event_type, to_channel);
    }
}
// --------------------------------- DMA channel ------------------------------------
const CFG_EN: u32 = 0b01_0000; // start a transfer
#[allow(dead_code)]
const CFG_CONT: u32 = 0b00_0001; // continuous mode
#[allow(dead_code)]
const CFG_SIZE_8: u32 = 0b00_0000; // 8-bit transfer
#[allow(dead_code)]
const CFG_SIZE_16: u32 = 0b00_0010; // 16-bit transfer
#[allow(dead_code)]
const CFG_SIZE_32: u32 = 0b00_0100; // 32-bit transfer
#[allow(dead_code)]
const CFG_CLEAR: u32 = 0b10_0000; // stop and clear all pending transfers
#[allow(dead_code)]
const CFG_PENDING: u32 = 0b10_0000; // on read, indicates a transfer pending
const CFG_SHADOW: u32 = 0b10_0000; // indicates a shadow transfer

#[repr(usize)]
pub enum Bank {
    Rx = 0,
    Tx = 0x10 / size_of::<u32>(),
    // woo dat special case...
    Custom = 0x20 / size_of::<u32>(),
}
impl Into<usize> for Bank {
    fn into(self) -> usize { self as usize }
}

/// Crate-local struct that defines the offset of registers in UDMA banks, as words.
#[repr(usize)]
pub enum DmaReg {
    Saddr = 0,
    Size = 1,
    Cfg = 2,
    #[allow(dead_code)]
    IntCfg = 3,
}
impl Into<usize> for DmaReg {
    fn into(self) -> usize { self as usize }
}

pub trait Udma {
    /// Every implementation of Udma has to implement the csr_mut() accessor
    fn csr_mut(&mut self) -> &mut CSR<u32>;
    /// Every implementation of Udma has to implement the csr() accessor
    fn csr(&self) -> &CSR<u32>;

    /// `bank` selects which UDMA bank is the target
    /// `buf` is a slice that points to the memory that is the target of the UDMA. Needs to be accessible
    ///    by the UDMA subsystem, e.g. in IFRAM0/IFRAM1 range, and is a *physical address* even in a
    ///    system running on virtual memory (!!!)
    /// `config` is a device-specific word that configures the DMA.
    ///
    /// Safety: the `buf` has to be allocated, length-checked, and in the range of memory
    /// that is valid for UDMA targets
    unsafe fn udma_enqueue<T>(&self, bank: Bank, buf: &[T], config: u32) {
        let bank_addr = self.csr().base().add(bank as usize);
        let buf_addr = buf.as_ptr() as u32;
        /*
        crate::println!(
            "udma_enqueue: @{:x}[{}]/{:x}",
            buf_addr,
            (buf.len() * size_of::<T>()) as u32,
            config | CFG_EN
        ); */
        bank_addr.add(DmaReg::Saddr.into()).write_volatile(buf_addr);
        bank_addr.add(DmaReg::Size.into()).write_volatile((buf.len() * size_of::<T>()) as u32);
        bank_addr.add(DmaReg::Cfg.into()).write_volatile(config | CFG_EN)
    }
    fn udma_can_enqueue(&self, bank: Bank) -> bool {
        // Safety: only safe when used in the context of UDMA registers.
        unsafe {
            (self.csr().base().add(bank as usize).add(DmaReg::Cfg.into()).read_volatile() & CFG_SHADOW) == 0
        }
    }
    fn udma_busy(&self, bank: Bank) -> bool {
        // Safety: only safe when used in the context of UDMA registers.
        unsafe { self.csr().base().add(bank as usize).add(DmaReg::Saddr.into()).read_volatile() != 0 }
    }
}

// ----------------------------------- UART ------------------------------------
#[repr(usize)]
enum UartReg {
    Status = 0,
    Setup = 1,
}
impl Into<usize> for UartReg {
    fn into(self) -> usize { self as usize }
}

#[repr(usize)]
pub enum UartChannel {
    Uart0 = 0,
    Uart1 = 1,
    Uart2 = 2,
    Uart3 = 3,
}
impl Into<usize> for UartChannel {
    fn into(self) -> usize { self as usize }
}

/// UDMA UART wrapper. Contains all the warts on top of the Channel abstraction.
pub struct Uart {
    /// This is assumed to point to the base of the peripheral's UDMA register set.
    csr: CSR<u32>,
    #[allow(dead_code)] // suppress warning with `std` is not selected
    ifram: IframRange,
}

/// Blanket implementations to access the CSR within UART. Needed because you can't
/// have default fields in traits: https://github.com/rust-lang/rfcs/pull/1546
impl Udma for Uart {
    fn csr_mut(&mut self) -> &mut CSR<u32> { &mut self.csr }

    fn csr(&self) -> &CSR<u32> { &self.csr }
}
/// The sum of UART_TX_BUF_SIZE + UART_RX_BUF_SIZE should be 4096.
const UART_TX_BUF_SIZE: usize = 2048;
const UART_RX_BUF_START: usize = UART_TX_BUF_SIZE;
const UART_RX_BUF_SIZE: usize = 2048;
const RX_BUF_DEPTH: usize = 1;
impl Uart {
    /// Configures for N81
    ///
    /// This function is `unsafe` because it can only be called after the
    /// global shared UDMA state has been set up to un-gate clocks and set up
    /// events.
    ///
    /// It is also `unsafe` on Drop because you have to remember to unmap
    /// the clock manually as well once the object is dropped...
    ///
    /// Allocates a 4096-deep buffer for tx/rx purposes: the first 2048 bytes
    /// are used for Tx, the second 2048 bytes for Rx. If this buffer size has
    /// to change, be sure to update the loader, as it takes this as an assumption
    /// since no IFRAM allocator is running at that time.
    #[cfg(feature = "std")]
    pub unsafe fn new(channel: UartChannel, baud: u32, clk_freq: u32) -> Self {
        assert!(UART_RX_BUF_SIZE + UART_TX_BUF_SIZE == 4096, "Configuration error in UDMA UART");
        let bank_addr = match channel {
            UartChannel::Uart0 => utra::udma_uart_0::HW_UDMA_UART_0_BASE,
            UartChannel::Uart1 => utra::udma_uart_1::HW_UDMA_UART_1_BASE,
            UartChannel::Uart2 => utra::udma_uart_2::HW_UDMA_UART_2_BASE,
            UartChannel::Uart3 => utra::udma_uart_3::HW_UDMA_UART_3_BASE,
        };
        let uart = xous::syscall::map_memory(
            xous::MemoryAddress::new(bank_addr),
            None,
            4096,
            xous::MemoryFlags::R | xous::MemoryFlags::W,
        )
        .expect("couldn't map serial port");

        // now setup the channel
        let csr = CSR::new(uart.as_mut_ptr() as *mut u32);

        let clk_counter: u32 = (clk_freq + baud / 2) / baud;
        // setup baud, bits, parity, etc.
        csr.base()
            .add(Bank::Custom.into())
            .add(UartReg::Setup.into())
            .write_volatile(0x0306 | (clk_counter << 16));

        Uart { csr, ifram: IframRange::request(UART_RX_BUF_SIZE + UART_TX_BUF_SIZE, None).unwrap() }
    }

    /// Gets a handle to the UART. Used for re-acquiring previously initialized
    /// UART hardware, such as from the loader booting into Xous
    ///
    /// Safety: only safe to call in the context of a previously initialized UART
    pub unsafe fn get_handle(csr_virt_addr: usize, udma_phys_addr: usize, udma_virt_addr: usize) -> Self {
        assert!(UART_RX_BUF_SIZE + UART_TX_BUF_SIZE == 4096, "Configuration error in UDMA UART");
        let csr = CSR::new(csr_virt_addr as *mut u32);
        Uart {
            csr,
            ifram: IframRange::from_raw_parts(
                udma_phys_addr,
                udma_virt_addr,
                UART_RX_BUF_SIZE + UART_TX_BUF_SIZE,
            ),
        }
    }

    pub fn set_baud(&self, baud: u32, clk_freq: u32) {
        let clk_counter: u32 = (clk_freq + baud / 2) / baud;
        // setup baud, bits, parity, etc.
        // safety: this is safe to call as long as the base address points at a valid UART.
        unsafe {
            self.csr
                .base()
                .add(Bank::Custom.into())
                .add(UartReg::Setup.into())
                .write_volatile(0x0306 | (clk_counter << 16));
        }
    }

    pub fn disable(&mut self) {
        self.wait_tx_done();
        // safe only in the context of a UART UDMA address
        unsafe {
            self.csr.base().add(Bank::Custom.into()).add(UartReg::Setup.into()).write_volatile(0x0050_0006);
        }
    }

    pub fn tx_busy(&self) -> bool {
        // safe only in the context of a UART UDMA address
        unsafe {
            (self.csr.base().add(Bank::Custom.into()).add(UartReg::Status.into()).read_volatile() & 1) != 0
        }
    }

    pub fn rx_busy(&self) -> bool {
        // safe only in the context of a UART UDMA address
        unsafe {
            (self.csr.base().add(Bank::Custom.into()).add(UartReg::Status.into()).read_volatile() & 2) != 0
        }
    }

    pub fn wait_tx_done(&self) {
        while self.udma_busy(Bank::Tx) {
            #[cfg(feature = "std")]
            xous::yield_slice();
        }
        while self.tx_busy() {}
    }

    pub fn wait_rx_done(&self) {
        while self.udma_busy(Bank::Rx) {
            #[cfg(feature = "std")]
            xous::yield_slice();
        }
    }

    /// `buf` is assumed to be a virtual address (in `std`), or a machine address
    /// (in baremetal mode). This function is safe because it will operate as intended
    /// within a given environment, so long as the `std` flag is applied correctly.
    ///
    /// When not in `std`, it's *also* assumed that `buf` is range-checked to be valid
    /// for the UDMA engine.
    ///
    /// returns: total length of bytes written
    pub fn write(&mut self, buf: &[u8]) -> usize {
        let mut writelen = 0;
        for chunk in buf.chunks(UART_TX_BUF_SIZE) {
            #[cfg(feature = "std")]
            {
                self.ifram.as_slice_mut()[..chunk.len()].copy_from_slice(chunk);
                // safety: the slice is in the physical range for the UDMA, and length-checked
                unsafe {
                    self.udma_enqueue(
                        Bank::Tx,
                        &self.ifram.as_phys_slice::<u8>()[..chunk.len()],
                        CFG_EN | CFG_SIZE_8,
                    );
                }
                writelen += chunk.len();
            }
            #[cfg(not(feature = "std"))]
            {
                self.ifram.as_slice_mut()[..chunk.len()].copy_from_slice(chunk);
                unsafe {
                    self.udma_enqueue(
                        Bank::Tx,
                        &self.ifram.as_phys_slice::<u8>()[..chunk.len()],
                        CFG_EN | CFG_SIZE_8,
                    );
                    writelen += chunk.len();
                }
            }

            self.wait_tx_done();
        }
        writelen
    }

    pub fn read(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(UART_RX_BUF_SIZE) {
            #[cfg(feature = "std")]
            unsafe {
                self.udma_enqueue(
                    Bank::Rx,
                    &self.ifram.as_phys_slice::<u8>()[UART_RX_BUF_START..UART_RX_BUF_START + chunk.len()],
                    CFG_EN | CFG_SIZE_8,
                );
            }
            #[cfg(not(feature = "std"))]
            unsafe {
                self.udma_enqueue(
                    Bank::Rx,
                    &self.ifram.as_phys_slice::<u8>()[UART_RX_BUF_START..UART_RX_BUF_START + chunk.len()],
                    CFG_EN | CFG_SIZE_8,
                );
            }
            self.wait_rx_done();
            #[cfg(feature = "std")]
            chunk.copy_from_slice(
                &self.ifram.as_slice::<u8>()[UART_RX_BUF_START..UART_RX_BUF_START + chunk.len()],
            );
            #[cfg(not(feature = "std"))]
            unsafe {
                chunk.copy_from_slice(
                    &self.ifram.as_phys_slice::<u8>()[UART_RX_BUF_START..UART_RX_BUF_START + chunk.len()],
                );
            }
        }
    }

    /// Call this to read one character on receiving an interrupt.
    ///
    /// Note that if the interrupt is not handled fast enough, characters are simply dropped.
    ///
    /// Returns actual number of bytes read (0 or 1).
    pub fn read_async(&mut self, c: &mut u8) -> usize {
        let bank_addr = unsafe { self.csr().base().add(Bank::Rx as usize) };
        // retrieve total bytes available
        let pending = unsafe { bank_addr.add(DmaReg::Size.into()).read_volatile() } as usize;

        // recover the pending byte. Hard-coded for case of RX_BUF_DEPTH == 1
        assert!(RX_BUF_DEPTH == 1, "Need to refactor buf recovery code if RX_BUF_DEPTH > 1");
        #[cfg(feature = "std")]
        {
            *c = self.ifram.as_slice::<u8>()[UART_RX_BUF_START];
        }
        #[cfg(not(feature = "std"))]
        unsafe {
            *c = self.ifram.as_phys_slice::<u8>()[UART_RX_BUF_START];
        }

        // queue the next round
        #[cfg(feature = "std")]
        unsafe {
            self.udma_enqueue(
                Bank::Rx,
                &self.ifram.as_phys_slice::<u8>()[UART_RX_BUF_START..UART_RX_BUF_START + RX_BUF_DEPTH],
                CFG_EN | CFG_CONT,
            );
        }
        #[cfg(not(feature = "std"))]
        unsafe {
            self.udma_enqueue(
                Bank::Rx,
                &self.ifram.as_phys_slice::<u8>()[UART_RX_BUF_START..UART_RX_BUF_START + RX_BUF_DEPTH],
                CFG_EN | CFG_CONT,
            );
        }

        pending
    }

    /// Call this to prime the system for async reads. This must be called at least once if any characters
    /// are ever to be received.
    pub fn setup_async_read(&mut self) {
        #[cfg(feature = "std")]
        unsafe {
            self.udma_enqueue(
                Bank::Rx,
                &self.ifram.as_phys_slice::<u8>()[UART_RX_BUF_START..UART_RX_BUF_START + RX_BUF_DEPTH],
                CFG_EN | CFG_CONT,
            );
        }
        #[cfg(not(feature = "std"))]
        unsafe {
            self.udma_enqueue(
                Bank::Rx,
                &self.ifram.as_phys_slice::<u8>()[UART_RX_BUF_START..UART_RX_BUF_START + RX_BUF_DEPTH],
                CFG_EN | CFG_CONT,
            );
        }
    }
}

#[derive(Debug)]
pub struct UartIrq {
    pub csr: CSR<u32>,
    #[cfg(feature = "std")]
    pub handlers: [Option<HandlerFn>; 4],
    #[cfg(feature = "std")]
    /// We can't claim the interrupt when the object is created, because the version we allocate
    /// inside `new()` is a temporary instance that exists on the stack. It's recommend that the
    /// caller put `UartIrq` inside a `Box` so that the location of the structure does not move
    /// around. Later on, when `register_handler()` is invoked, the address of `self` is used to
    /// pass into the handler. It is important that the caller ensures that `self` does not move around.
    interrupt_claimed: bool,
}
impl UartIrq {
    #[cfg(feature = "std")]
    pub fn new() -> Self {
        let uart = xous::syscall::map_memory(
            xous::MemoryAddress::new(HW_IRQARRAY5_BASE),
            None,
            4096,
            xous::MemoryFlags::R | xous::MemoryFlags::W,
        )
        .expect("couldn't map uart IRQ control");
        Self {
            csr: CSR::new(uart.as_ptr() as *mut u32),
            handlers: [None, None, None, None],
            interrupt_claimed: false,
        }
    }

    #[cfg(not(feature = "std"))]
    pub fn new() -> Self {
        use riscv::register::vexriscv::mim;
        mim::write(mim::read() | (1 << utra::irqarray5::IRQARRAY5_IRQ));
        Self { csr: CSR::new(HW_IRQARRAY5_BASE as *mut u32) }
    }

    pub fn rx_irq_ena(&mut self, channel: UartChannel, enable: bool) {
        let val = if enable { 1 } else { 0 };
        match channel {
            UartChannel::Uart0 => self.csr.rmwf(utra::irqarray5::EV_ENABLE_UART0_RX, val),
            UartChannel::Uart1 => self.csr.rmwf(utra::irqarray5::EV_ENABLE_UART1_RX, val),
            UartChannel::Uart2 => self.csr.rmwf(utra::irqarray5::EV_ENABLE_UART2_RX, val),
            UartChannel::Uart3 => self.csr.rmwf(utra::irqarray5::EV_ENABLE_UART3_RX, val),
        }
    }

    #[cfg(feature = "std")]
    /// This needs to be invoked from a Pin'd Box wrapper of the UartIrq structure. Here is how the
    /// pattern looks:
    ///
    /// ```rust
    /// let mut uart_irq = Box::pin(cramium_hal::udma::UartIrq::new());
    /// Pin::as_mut(&mut uart_irq).register_handler(udma::UartChannel::Uart1, uart_handler);
    /// ```
    ///
    /// What this does is bind a `UartIrq` instance to an address in the heap (via Box), and
    /// marks that address as non-moveable (via Pin), ensuring that the `register_handler` call's
    /// view of `self` stays around forever.
    ///
    /// Note: this does not also enable the interrupt channel, it just registers the handler
    ///
    /// Safety: the function is only safe to use if `self` has a `static` lifetime, that is, the
    /// `UartIrq` object will live the entire duration of the OS. If the object is destroyed,
    /// the IRQ handler will point to an invalid location and the system will crash. In general,
    /// we don't intend this kind of behavior, so we don't implement a `Drop` because simply
    /// de-allocating the interrupt handler on an accidental Drop is probably not intentional
    /// and can lead to even more confusing/harder-to-debug faults, i.e., the system won't crash,
    /// but it will simply stop responding to interrupts. As a philosophical point, if an unregister behavior
    /// is desired, it should be explicit.
    pub unsafe fn register_handler(
        mut self: std::pin::Pin<&mut Self>,
        channel: UartChannel,
        handler: HandlerFn,
    ) {
        if !self.interrupt_claimed {
            xous::claim_interrupt(
                utra::irqarray5::IRQARRAY5_IRQ,
                main_uart_handler,
                self.as_ref().get_ref() as *const UartIrq as *mut usize,
            )
            .expect("couldn't claim UART IRQ channel");
            self.interrupt_claimed = true;
        }

        self.handlers[channel as usize] = Some(handler);
    }
}

pub type HandlerFn = fn(usize, *mut usize);

#[cfg(feature = "std")]
fn main_uart_handler(irq_no: usize, arg: *mut usize) {
    // check ev_pending and dispatch handlers based on that
    let uartirq = unsafe { &mut *(arg as *mut UartIrq) };
    let pending = uartirq.csr.r(utra::irqarray5::EV_PENDING);
    if pending & uartirq.csr.ms(utra::irqarray5::EV_PENDING_UART0_RX, 1) != 0 {
        if let Some(h) = uartirq.handlers[0] {
            h(irq_no, arg);
        }
    }
    if pending & uartirq.csr.ms(utra::irqarray5::EV_PENDING_UART1_RX, 1) != 0 {
        if let Some(h) = uartirq.handlers[1] {
            h(irq_no, arg);
        }
    }
    if pending & uartirq.csr.ms(utra::irqarray5::EV_PENDING_UART2_RX, 1) != 0 {
        if let Some(h) = uartirq.handlers[2] {
            h(irq_no, arg);
        }
    }
    if pending & uartirq.csr.ms(utra::irqarray5::EV_PENDING_UART3_RX, 1) != 0 {
        if let Some(h) = uartirq.handlers[3] {
            h(irq_no, arg);
        }
    }
    // note that this will also clear other spurious interrupts without issuing a warning.
    uartirq.csr.wo(utra::irqarray5::EV_PENDING, pending);
}

// ----------------------------------- SPIM ------------------------------------

/// The SPIM implementation for UDMA does reg-ception, in that they bury
/// a register set inside a register set. The registers are only accessible by,
/// surprise, DMA. The idea behind this is you can load a bunch of commands into
/// memory and just DMA them to the control interface. Sure, cool idea bro.
///
/// Anyways, the autodoc system is unable to extract the register
/// formats for the SPIM. Instead, we have to create a set of hand-crafted
/// structures to deal with this.

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum SpimClkPol {
    LeadingEdgeRise = 0,
    LeadingEdgeFall = 1,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum SpimClkPha {
    CaptureOnLeading = 0,
    CaptureOnTrailing = 1,
}
#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum SpimCs {
    Cs0 = 0,
    Cs1 = 1,
    Cs2 = 2,
    Cs3 = 3,
}
#[repr(u32)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SpimMode {
    Standard = 0,
    Quad = 1,
}
#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum SpimByteAlign {
    Enable = 0,
    Disable = 1,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum SpimCheckType {
    Allbits = 0,
    OnlyOnes = 1,
    OnlyZeros = 2,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum SpimEventGen {
    Disabled = 0,
    Enabled = 1,
}
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum SpimWordsPerXfer {
    Words1 = 0,
    Words2 = 1,
    Words4 = 2,
}
#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum SpimEndian {
    MsbFirst = 0,
    LsbFirst = 1,
}
#[derive(Copy, Clone)]
pub enum SpimWaitType {
    Event(EventChannel),
    Cycles(u8),
}
#[derive(Copy, Clone)]
pub enum SpimCmd {
    /// pol, pha, clkdiv
    Config(SpimClkPol, SpimClkPha, u8),
    StartXfer(SpimCs),
    /// mode, cmd_size (5 bits), command value, left-aligned
    SendCmd(SpimMode, u8, u16),
    /// mode, number of address bits (5 bits)
    SendAddr(SpimMode, u8),
    /// number of cycles (5 bits)
    Dummy(u8),
    /// Wait on an event. Note EventChannel coding needs interpretation prior to use.
    /// type of wait, channel, cycle count
    Wait(SpimWaitType),
    /// mode, words per xfer, bits per word, endianness, number of words to send
    TxData(SpimMode, SpimWordsPerXfer, u8, SpimEndian, u32),
    /// mode, words per xfer, bits per word, endianness, number of words to receive
    RxData(SpimMode, SpimWordsPerXfer, u8, SpimEndian, u32),
    /// repeat count
    RepeatNextCmd(u16),
    EndXfer(SpimEventGen),
    EndRepeat,
    /// mode, use byte alignment, check type, size of comparison (4 bits), comparison data
    RxCheck(SpimMode, SpimByteAlign, SpimCheckType, u8, u16),
    /// use byte alignment, size of data
    FullDuplex(SpimByteAlign, u16),
}
impl Into<u32> for SpimCmd {
    fn into(self) -> u32 {
        match self {
            SpimCmd::Config(pol, pha, div) => 0 << 28 | (pol as u32) << 9 | (pha as u32) << 8 | div as u32,
            SpimCmd::StartXfer(cs) => 1 << 28 | cs as u32,
            SpimCmd::SendCmd(mode, size, cmd) => {
                2 << 28 | (mode as u32) << 27 | ((size - 1) as u32 & 0x1F) << 16 | cmd as u32
            }
            SpimCmd::SendAddr(mode, size) => 3 << 28 | (mode as u32) << 27 | (size as u32 & 0x1F) << 16,
            SpimCmd::Dummy(cycles) => 4 << 28 | (cycles as u32 & 0x1F) << 16,
            SpimCmd::Wait(wait_type) => {
                let wait_code = match wait_type {
                    SpimWaitType::Event(EventChannel::Channel0) => 0,
                    SpimWaitType::Event(EventChannel::Channel1) => 1,
                    SpimWaitType::Event(EventChannel::Channel2) => 2,
                    SpimWaitType::Event(EventChannel::Channel3) => 3,
                    SpimWaitType::Cycles(cyc) => cyc as u32 | 0x1_00,
                };
                5 << 28 | wait_code
            }
            SpimCmd::TxData(mode, words_per_xfer, bits_per_word, endian, len) => {
                6 << 28
                    | (mode as u32) << 27
                    | ((words_per_xfer as u32) & 0x3) << 21
                    | (bits_per_word as u32 - 1) << 16
                    | (len as u32 - 1)
                    | (endian as u32) << 26
            }
            SpimCmd::RxData(mode, words_per_xfer, bits_per_word, endian, len) => {
                7 << 28
                    | (mode as u32) << 27
                    | ((words_per_xfer as u32) & 0x3) << 21
                    | (bits_per_word as u32 - 1) << 16
                    | (len as u32 - 1)
                    | (endian as u32) << 26
            }
            SpimCmd::RepeatNextCmd(count) => 8 << 28 | count as u32,
            SpimCmd::EndXfer(event) => 9 << 28 | event as u32,
            SpimCmd::EndRepeat => 10 << 28,
            SpimCmd::RxCheck(mode, align, check_type, size, data) => {
                11 << 28
                    | (mode as u32) << 27
                    | (align as u32) << 26
                    | (check_type as u32) << 24
                    | (size as u32 & 0xF) << 16
                    | data as u32
            }
            SpimCmd::FullDuplex(align, len) => 12 << 28 | (align as u32) << 26 | len as u32,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SpimChannel {
    Channel0,
    Channel1,
    Channel2,
    Channel3,
}
#[derive(Debug)]
pub struct Spim {
    csr: CSR<u32>,
    cs: SpimCs,
    sot_wait: u8,
    eot_wait: u8,
    event_channel: Option<EventChannel>,
    mode: SpimMode,
    _align: SpimByteAlign,
    pub ifram: IframRange,
    // starts at the base of ifram range
    pub tx_buf_len_bytes: usize,
    // immediately after the tx buf len
    pub rx_buf_len_bytes: usize,
    dummy_cycles: u8,
    endianness: SpimEndian,
}

// length of the command buffer
const SPIM_CMD_BUF_LEN_BYTES: usize = 16;

impl Udma for Spim {
    fn csr_mut(&mut self) -> &mut CSR<u32> { &mut self.csr }

    fn csr(&self) -> &CSR<u32> { &self.csr }
}

impl Spim {
    /// This function is `unsafe` because it can only be called after the
    /// global shared UDMA state has been set up to un-gate clocks and set up
    /// events.
    ///
    /// It is also `unsafe` on Drop because you have to remember to unmap
    /// the clock manually as well once the object is dropped...
    ///
    /// Return: the function can return None if it can't allocate enough memory
    /// for the requested tx/rx length.
    #[cfg(feature = "std")]
    pub unsafe fn new(
        channel: SpimChannel,
        spi_clk_freq: u32,
        sys_clk_freq: u32,
        pol: SpimClkPol,
        pha: SpimClkPha,
        chip_select: SpimCs,
        // cycles to wait between CS assert and data start
        sot_wait: u8,
        // cycles to wait after data stop and CS de-assert
        eot_wait: u8,
        event_channel: Option<EventChannel>,
        max_tx_len_bytes: usize,
        max_rx_len_bytes: usize,
        dummy_cycles: Option<u8>,
    ) -> Option<Self> {
        // this is a hardware limit - the DMA pointer is only this long!
        assert!(max_tx_len_bytes < 65536);
        assert!(max_rx_len_bytes < 65536);
        // now setup the channel
        let base_addr = match channel {
            SpimChannel::Channel0 => utra::udma_spim_0::HW_UDMA_SPIM_0_BASE,
            SpimChannel::Channel1 => utra::udma_spim_1::HW_UDMA_SPIM_1_BASE,
            SpimChannel::Channel2 => utra::udma_spim_2::HW_UDMA_SPIM_2_BASE,
            SpimChannel::Channel3 => utra::udma_spim_3::HW_UDMA_SPIM_3_BASE,
        };
        let csr_range = xous::syscall::map_memory(
            xous::MemoryAddress::new(base_addr),
            None,
            4096,
            xous::MemoryFlags::R | xous::MemoryFlags::W,
        )
        .expect("couldn't map serial port");
        let csr = CSR::new(csr_range.as_mut_ptr() as *mut u32);

        let clk_div = sys_clk_freq / (2 * spi_clk_freq);
        // make this a hard panic -- you'll find out at runtime that you f'd up
        // but at least you find out.
        assert!(clk_div < 256, "SPI clock divider is out of range");

        let mut reqlen = max_tx_len_bytes + max_rx_len_bytes + SPIM_CMD_BUF_LEN_BYTES;
        if reqlen % 4096 != 0 {
            // round up to the nearest page size
            reqlen = (reqlen + 4096) & !4095;
        }
        if let Some(ifram) = IframRange::request(reqlen, None) {
            let mut spim = Spim {
                csr,
                cs: chip_select,
                sot_wait,
                eot_wait,
                event_channel,
                _align: SpimByteAlign::Disable,
                mode: SpimMode::Standard,
                ifram,
                tx_buf_len_bytes: max_tx_len_bytes,
                rx_buf_len_bytes: max_rx_len_bytes,
                dummy_cycles: dummy_cycles.unwrap_or(0),
                endianness: SpimEndian::MsbFirst,
            };
            // setup the interface using a UDMA command
            spim.send_cmd_list(&[SpimCmd::Config(pol, pha, clk_div as u8)]);

            Some(spim)
        } else {
            None
        }
    }

    /// This function is `unsafe` because it can only be called after the
    /// global shared UDMA state has been set up to un-gate clocks and set up
    /// events.
    ///
    /// It is also `unsafe` on Drop because you have to remember to unmap
    /// the clock manually as well once the object is dropped...
    ///
    /// Return: the function can return None if it can't allocate enough memory
    /// for the requested tx/rx length.
    pub unsafe fn new_with_ifram(
        channel: SpimChannel,
        spi_clk_freq: u32,
        sys_clk_freq: u32,
        pol: SpimClkPol,
        pha: SpimClkPha,
        chip_select: SpimCs,
        // cycles to wait between CS assert and data start
        sot_wait: u8,
        // cycles to wait after data stop and CS de-assert
        eot_wait: u8,
        event_channel: Option<EventChannel>,
        max_tx_len_bytes: usize,
        max_rx_len_bytes: usize,
        dummy_cycles: Option<u8>,
        mode: Option<SpimMode>,
        ifram: IframRange,
    ) -> Self {
        // this is a hardware limit - the DMA pointer is only this long!
        assert!(max_tx_len_bytes < 65536);
        assert!(max_rx_len_bytes < 65536);
        // now setup the channel
        let base_addr = match channel {
            SpimChannel::Channel0 => utra::udma_spim_0::HW_UDMA_SPIM_0_BASE,
            SpimChannel::Channel1 => utra::udma_spim_1::HW_UDMA_SPIM_1_BASE,
            SpimChannel::Channel2 => utra::udma_spim_2::HW_UDMA_SPIM_2_BASE,
            SpimChannel::Channel3 => utra::udma_spim_3::HW_UDMA_SPIM_3_BASE,
        };
        #[cfg(target_os = "xous")]
        let csr_range = xous::syscall::map_memory(
            xous::MemoryAddress::new(base_addr),
            None,
            4096,
            xous::MemoryFlags::R | xous::MemoryFlags::W,
        )
        .expect("couldn't map serial port");
        #[cfg(target_os = "xous")]
        let csr = CSR::new(csr_range.as_mut_ptr() as *mut u32);
        #[cfg(not(target_os = "xous"))]
        let csr = CSR::new(base_addr as *mut u32);

        let clk_div = sys_clk_freq / (2 * spi_clk_freq);
        // make this a hard panic -- you'll find out at runtime that you f'd up
        // but at least you find out.
        assert!(clk_div < 256, "SPI clock divider is out of range");

        let mut spim = Spim {
            csr,
            cs: chip_select,
            sot_wait,
            eot_wait,
            event_channel,
            _align: SpimByteAlign::Disable,
            mode: mode.unwrap_or(SpimMode::Standard),
            ifram,
            tx_buf_len_bytes: max_tx_len_bytes,
            rx_buf_len_bytes: max_rx_len_bytes,
            dummy_cycles: dummy_cycles.unwrap_or(0),
            endianness: SpimEndian::MsbFirst,
        };
        // setup the interface using a UDMA command
        spim.send_cmd_list(&[SpimCmd::Config(pol, pha, clk_div as u8)]);

        spim
    }

    /// For creating a clone to the current SPIM handle passed through a thread.
    ///
    /// Safety: can only be used on devices that are static for the life of the OS. Also, does nothing
    /// to prevent races/contention for the underlying device. The main reason this is introduced is
    /// to facilitate a panic handler for the graphics frame buffer, where we're about to kill the OS
    /// anyways: we don't care about soundness guarantees after this point.
    ///
    /// Note that the endianness is set to MSB first by default.
    pub unsafe fn from_raw_parts(
        csr: usize,
        cs: SpimCs,
        sot_wait: u8,
        eot_wait: u8,
        event_channel: Option<EventChannel>,
        mode: SpimMode,
        _align: SpimByteAlign,
        ifram: IframRange,
        tx_buf_len_bytes: usize,
        rx_buf_len_bytes: usize,
        dummy_cycles: u8,
    ) -> Self {
        Spim {
            csr: CSR::new(csr as *mut u32),
            cs,
            sot_wait,
            eot_wait,
            event_channel,
            _align,
            mode,
            ifram,
            tx_buf_len_bytes,
            rx_buf_len_bytes,
            dummy_cycles,
            endianness: SpimEndian::MsbFirst,
        }
    }

    /// Blows a SPIM structure into parts that can be sent across a thread boundary.
    ///
    /// Safety: this is only safe because the *mut u32 for the CSR doesn't change, because it's tied to
    /// a piece of hardware, not some arbitrary block of memory.
    pub unsafe fn into_raw_parts(
        &self,
    ) -> (usize, SpimCs, u8, u8, Option<EventChannel>, SpimMode, SpimByteAlign, IframRange, usize, usize, u8)
    {
        (
            self.csr.base() as usize,
            self.cs,
            self.sot_wait,
            self.eot_wait,
            self.event_channel,
            self.mode,
            self._align,
            IframRange {
                phys_range: self.ifram.phys_range,
                virt_range: self.ifram.virt_range,
                conn: self.ifram.conn,
            },
            self.tx_buf_len_bytes,
            self.rx_buf_len_bytes,
            self.dummy_cycles,
        )
    }

    /// Note that endianness is disregarded in the case that the channel is being used to talk to
    /// a memory device, because the endianness is always MsbFirst.
    pub fn set_endianness(&mut self, endianness: SpimEndian) { self.endianness = endianness; }

    pub fn get_endianness(&self) -> SpimEndian { self.endianness }

    /// The command buf is *always* a `u32`; so tie the type down here.
    fn cmd_buf_mut(&mut self) -> &mut [u32] {
        &mut self.ifram.as_slice_mut()[(self.tx_buf_len_bytes + self.rx_buf_len_bytes) / size_of::<u32>()
            ..(self.tx_buf_len_bytes + self.rx_buf_len_bytes + SPIM_CMD_BUF_LEN_BYTES) / size_of::<u32>()]
    }

    unsafe fn cmd_buf_phys(&self) -> &[u32] {
        &self.ifram.as_phys_slice()[(self.tx_buf_len_bytes + self.rx_buf_len_bytes) / size_of::<u32>()
            ..(self.tx_buf_len_bytes + self.rx_buf_len_bytes + SPIM_CMD_BUF_LEN_BYTES) / size_of::<u32>()]
    }

    pub fn rx_buf<T: UdmaWidths>(&mut self) -> &[T] {
        &self.ifram.as_slice()[(self.tx_buf_len_bytes) / size_of::<T>()
            ..(self.tx_buf_len_bytes + self.rx_buf_len_bytes) / size_of::<T>()]
    }

    pub unsafe fn rx_buf_phys<T: UdmaWidths>(&self) -> &[T] {
        &self.ifram.as_phys_slice()[(self.tx_buf_len_bytes) / size_of::<T>()
            ..(self.tx_buf_len_bytes + self.rx_buf_len_bytes) / size_of::<T>()]
    }

    pub fn tx_buf_mut<T: UdmaWidths>(&mut self) -> &mut [T] {
        &mut self.ifram.as_slice_mut()[..self.tx_buf_len_bytes / size_of::<T>()]
    }

    pub unsafe fn tx_buf_phys<T: UdmaWidths>(&self) -> &[T] {
        &self.ifram.as_phys_slice()[..self.tx_buf_len_bytes / size_of::<T>()]
    }

    fn send_cmd_list(&mut self, cmds: &[SpimCmd]) {
        for cmd_chunk in cmds.chunks(SPIM_CMD_BUF_LEN_BYTES / size_of::<u32>()) {
            for (src, dst) in cmd_chunk.iter().zip(self.cmd_buf_mut().iter_mut()) {
                *dst = (*src).into();
            }
            // safety: this is safe because the cmd_buf_phys() slice is passed to a function that only
            // uses it as a base/bounds reference and it will not actually access the data.
            unsafe {
                self.udma_enqueue(
                    Bank::Custom,
                    &self.cmd_buf_phys()[..cmd_chunk.len()],
                    CFG_EN | CFG_SIZE_32,
                );
            }
        }
    }

    pub fn is_tx_busy(&self) -> bool { self.udma_busy(Bank::Tx) || self.udma_busy(Bank::Custom) }

    pub fn tx_data_await(&self, _use_yield: bool) {
        while self.is_tx_busy() {
            #[cfg(feature = "std")]
            if _use_yield {
                xous::yield_slice();
            }
        }
    }

    /// `tx_data_async` will queue a data buffer into the SPIM interface and return as soon as the enqueue
    /// is completed (which can be before the transmission is actually done). The function may partially
    /// block, however, if the size of the buffer to be sent is larger than the largest allowable DMA
    /// transfer. In this case, it will block until the last chunk that can be transferred without
    /// blocking.
    pub fn tx_data_async<T: UdmaWidths + Copy>(&mut self, data: &[T], use_cs: bool, eot_event: bool) {
        unsafe {
            self.tx_data_async_inner(Some(data), None, use_cs, eot_event);
        }
    }

    /// `tx_data_async_from_parts` does a similar function as `tx_data_async`, but it expects that the
    /// data to send is already copied into the DMA buffer. In this case, no copying is done, and the
    /// `(start, len)` pair is used to specify the beginning and the length of the data to send that is
    /// already resident in the DMA buffer.
    ///
    /// Safety:
    ///   - Only safe to use when the data has already been copied into the DMA buffer, and the size and len
    ///     fields are within bounds.
    pub unsafe fn tx_data_async_from_parts<T: UdmaWidths + Copy>(
        &mut self,
        start: usize,
        len: usize,
        use_cs: bool,
        eot_event: bool,
    ) {
        self.tx_data_async_inner(None::<&[T]>, Some((start, len)), use_cs, eot_event);
    }

    /// This is the inner implementation of the two prior calls. A lot of the boilerplate is the same,
    /// the main difference is just whether the passed data shall be copied or not.
    ///
    /// Panics: Panics if both `data` and `parts` are `None`. If both are `Some`, `data` will take precedence.
    unsafe fn tx_data_async_inner<T: UdmaWidths + Copy>(
        &mut self,
        data: Option<&[T]>,
        parts: Option<(usize, usize)>,
        use_cs: bool,
        eot_event: bool,
    ) {
        let bits_per_xfer = size_of::<T>() * 8;
        let total_words = if let Some(data) = data {
            data.len()
        } else if let Some((_start, len)) = parts {
            len
        } else {
            // I can't figure out how to wrap a... &[T] in an enum? A slice of a type of trait
            // seems to need some sort of `dyn` keyword plus other stuff that is a bit heavy for
            // a function that is private (note the visibility on this function). Handling this
            // instead with a runtime check-to-panic.
            panic!("Inner function was set up with incorrect arguments");
        };
        let mut words_sent: usize = 0;

        if use_cs {
            // ensure any previous transaction is completed
            while self.udma_busy(Bank::Custom) || self.udma_busy(Bank::Tx) {}
            if self.sot_wait == 0 {
                self.send_cmd_list(&[SpimCmd::StartXfer(self.cs)])
            } else {
                self.send_cmd_list(&[
                    SpimCmd::StartXfer(self.cs),
                    SpimCmd::Wait(SpimWaitType::Cycles(self.sot_wait)),
                ])
            }
            // wait for CS to assert
            while self.udma_busy(Bank::Custom) {}
        }
        let mut one_shot = false;
        let evt = if eot_event { SpimEventGen::Enabled } else { SpimEventGen::Disabled };
        while words_sent < total_words {
            // determine the valid length of data we could send
            let tx_len = (total_words - words_sent).min(self.tx_buf_len_bytes);
            // setup the command list for data to send
            let cmd_list_oneshot = [
                SpimCmd::TxData(
                    self.mode,
                    SpimWordsPerXfer::Words1,
                    bits_per_xfer as u8,
                    self.get_endianness(),
                    tx_len as u32,
                ),
                SpimCmd::EndXfer(evt),
            ];
            let cmd_list_repeated = [SpimCmd::TxData(
                self.mode,
                SpimWordsPerXfer::Words1,
                bits_per_xfer as u8,
                self.get_endianness(),
                tx_len as u32,
            )];
            if tx_len == total_words && use_cs {
                one_shot = true;
                self.send_cmd_list(&cmd_list_oneshot);
            } else {
                self.send_cmd_list(&cmd_list_repeated);
            }
            let cfg_size = match size_of::<T>() {
                1 => CFG_SIZE_8,
                2 => CFG_SIZE_16,
                4 => CFG_SIZE_32,
                _ => panic!("Illegal size of UdmaWidths: should not be possible"),
            };
            if let Some(data) = data {
                for (src, dst) in
                    data[words_sent..words_sent + tx_len].iter().zip(self.tx_buf_mut().iter_mut())
                {
                    *dst = *src;
                }
                // safety: this is safe because tx_buf_phys() slice is only used as a base/bounds reference
                unsafe { self.udma_enqueue(Bank::Tx, &self.tx_buf_phys::<T>()[..tx_len], CFG_EN | cfg_size) }
            } else if let Some((start, _len)) = parts {
                // safety: this is safe because tx_buf_phys() slice is only used as a base/bounds reference
                // This will correctly panic if the size of the data to be sent is larger than the physical
                // tx_buf.
                unsafe {
                    self.udma_enqueue(
                        Bank::Tx,
                        &self.tx_buf_phys::<T>()[(start + words_sent)..(start + words_sent + tx_len)],
                        CFG_EN | cfg_size,
                    )
                }
            } // the else clause "shouldn't happen" because of the runtime check up top!
            words_sent += tx_len;

            // wait until the transfer is done before doing the next iteration, if there is a next iteration
            // last iteration falls through without waiting...
            if words_sent < total_words {
                while self.udma_busy(Bank::Tx) {
                    #[cfg(feature = "std")]
                    xous::yield_slice();
                }
            }
        }
        if use_cs && !one_shot {
            // wait for all data to transmit before de-asserting CS
            while self.udma_busy(Bank::Tx) {}
            if self.eot_wait == 0 {
                self.send_cmd_list(&[SpimCmd::EndXfer(evt)])
            } else {
                self.send_cmd_list(&[
                    SpimCmd::Wait(SpimWaitType::Cycles(self.eot_wait)),
                    SpimCmd::EndXfer(evt),
                ])
            }
            while self.udma_busy(Bank::Custom) {}
        }
    }

    pub fn rx_data<T: UdmaWidths + Copy>(&mut self, _rx_data: &mut [T], _cs: Option<SpimCs>) {
        todo!("Not yet done...template off of tx_data, but we need a test target before we can do this");
    }

    /// Activate is the logical sense, not the physical sense. To be clear: `true` causes CS to go low.
    fn mem_cs(&mut self, activate: bool) {
        if activate {
            if self.sot_wait == 0 {
                self.send_cmd_list(&[SpimCmd::StartXfer(self.cs)])
            } else {
                self.send_cmd_list(&[
                    SpimCmd::StartXfer(self.cs),
                    SpimCmd::Wait(SpimWaitType::Cycles(self.sot_wait)),
                ])
            }
        } else {
            let evt =
                if self.event_channel.is_some() { SpimEventGen::Enabled } else { SpimEventGen::Disabled };
            if self.eot_wait == 0 {
                self.send_cmd_list(&[SpimCmd::EndXfer(evt)])
            } else {
                self.send_cmd_list(&[
                    SpimCmd::Wait(SpimWaitType::Cycles(self.eot_wait)),
                    SpimCmd::EndXfer(evt),
                ])
            }
        }
    }

    fn mem_send_cmd(&mut self, cmd: u8) {
        let cmd_list = [SpimCmd::SendCmd(self.mode, 8, cmd as u16)];
        self.send_cmd_list(&cmd_list);
        while self.udma_busy(Bank::Custom) {
            #[cfg(feature = "std")]
            xous::yield_slice();
        }
    }

    pub fn mem_read_id_flash(&mut self) -> u32 {
        self.mem_cs(true);

        // send the RDID command
        match self.mode {
            SpimMode::Standard => self.mem_send_cmd(0x9F),
            SpimMode::Quad => self.mem_send_cmd(0xAF),
        }

        // read back the ID result
        let cmd_list = [SpimCmd::RxData(self.mode, SpimWordsPerXfer::Words1, 8, SpimEndian::MsbFirst, 3)];
        self.send_cmd_list(&cmd_list);
        // safety: this is safe because rx_buf_phys() slice is only used as a base/bounds reference
        unsafe { self.udma_enqueue(Bank::Rx, &self.rx_buf_phys::<u8>()[..3], CFG_EN | CFG_SIZE_8) };
        while self.udma_busy(Bank::Rx) {
            #[cfg(feature = "std")]
            xous::yield_slice();
        }

        let ret = u32::from_le_bytes([self.rx_buf()[0], self.rx_buf()[1], self.rx_buf()[2], 0x0]);

        self.mem_cs(false);
        ret
    }

    /// Side-effects: unsets QPI mode if it was previously set
    pub fn mem_read_id_ram(&mut self) -> u32 {
        self.mem_cs(true);

        // send the RDID command
        self.mem_send_cmd(0x9F);

        // read back the ID result
        // The ID requires 24 bits "dummy" address field, then followed by 2 bytes ID + KGD, and then
        // 48 bits of unique ID -- we only retrieve the top 16 of that here.
        let cmd_list = [SpimCmd::RxData(self.mode, SpimWordsPerXfer::Words1, 8, SpimEndian::MsbFirst, 7)];
        self.send_cmd_list(&cmd_list);
        // safety: this is safe because rx_buf_phys() slice is only used as a base/bounds reference
        unsafe { self.udma_enqueue(Bank::Rx, &self.rx_buf_phys::<u8>()[..7], CFG_EN | CFG_SIZE_8) };
        while self.udma_busy(Bank::Rx) {
            #[cfg(feature = "std")]
            xous::yield_slice();
        }

        let ret =
            u32::from_le_bytes([self.rx_buf()[3], self.rx_buf()[4], self.rx_buf()[5], self.rx_buf()[6]]);

        self.mem_cs(false);
        ret
    }

    pub fn mem_qpi_mode(&mut self, activate: bool) {
        self.mem_cs(true);
        if activate {
            self.mem_send_cmd(0x35);
        } else {
            self.mode = SpimMode::Quad; // pre-assumes quad mode
            self.mem_send_cmd(0xF5);
        }
        self.mem_cs(false);
        // change the mode only after the command has been sent
        if activate {
            self.mode = SpimMode::Quad;
        } else {
            self.mode = SpimMode::Standard;
        }
    }

    /// Side-effects: unsets QPI mode if it was previously set
    /// TODO: this does not seem to work. Setting it causes some strange behaviors
    /// on reads (but QE mode is enabled, so something must have worked). This
    /// needs to be looked into more. Oddly enough, it looks "fine" on the logic
    /// analyzer when I checked it early on, but obviously something is not right.
    pub fn mem_write_status_register(&mut self, status: u8, config: u8) {
        if self.mode != SpimMode::Standard {
            self.mem_qpi_mode(false);
        }
        self.mem_cs(true);
        self.mem_send_cmd(0x1);
        // setup the command list for data to send
        let cmd_list =
            [SpimCmd::TxData(self.mode, SpimWordsPerXfer::Words1, 8 as u8, SpimEndian::MsbFirst, 2 as u32)];
        self.send_cmd_list(&cmd_list);
        self.tx_buf_mut()[..2].copy_from_slice(&[status, config]);
        // safety: this is safe because tx_buf_phys() slice is only used as a base/bounds reference
        unsafe { self.udma_enqueue(Bank::Tx, &self.tx_buf_phys::<u8>()[..2], CFG_EN | CFG_SIZE_8) }

        while self.udma_busy(Bank::Tx) {
            #[cfg(feature = "std")]
            xous::yield_slice();
        }
        self.mem_cs(false);
    }

    /// Note that `use_yield` is disallowed in interrupt contexts (e.g. swapper)
    pub fn mem_read(&mut self, addr: u32, buf: &mut [u8], _use_yield: bool) -> bool {
        // divide into buffer-sized chunks + repeat cycle on each buffer increment
        // this is because the size of the buffer is meant to represent the limit of the
        // target device's memory page (i.e., the point at which you'd wrap when reading)
        let mut offset = 0;
        let mut timeout = 0;
        let mut success = true;
        for chunk in buf.chunks_mut(self.rx_buf_len_bytes) {
            let chunk_addr = addr as usize + offset;
            let addr_plus_dummy = (24 / 8) + self.dummy_cycles / 2;
            let cmd_list = [
                SpimCmd::SendCmd(self.mode, 8, 0xEB),
                SpimCmd::TxData(
                    self.mode,
                    SpimWordsPerXfer::Words1,
                    8 as u8,
                    SpimEndian::MsbFirst,
                    addr_plus_dummy as u32,
                ),
            ];
            let a = chunk_addr.to_be_bytes();
            self.tx_buf_mut()[..3].copy_from_slice(&[a[1], a[2], a[3]]);
            // the remaining bytes are junk
            self.tx_buf_mut()[3..6].copy_from_slice(&[0xFFu8, 0xFFu8, 0xFFu8]);
            self.mem_cs(true);
            self.send_cmd_list(&cmd_list);
            // safety: this is safe because tx_buf_phys() slice is only used as a base/bounds reference
            unsafe {
                self.udma_enqueue(
                    Bank::Tx,
                    &self.tx_buf_phys::<u8>()[..addr_plus_dummy as usize],
                    CFG_EN | CFG_SIZE_8,
                )
            }
            let rd_cmd = [SpimCmd::RxData(
                self.mode,
                SpimWordsPerXfer::Words1,
                8,
                SpimEndian::MsbFirst,
                chunk.len() as u32,
            )];
            while self.udma_busy(Bank::Tx) {
                #[cfg(feature = "std")]
                if _use_yield {
                    xous::yield_slice();
                }
            }
            self.send_cmd_list(&rd_cmd);
            // safety: this is safe because rx_buf_phys() slice is only used as a base/bounds reference
            unsafe {
                self.udma_enqueue(Bank::Rx, &self.rx_buf_phys::<u8>()[..chunk.len()], CFG_EN | CFG_SIZE_8)
            };
            while self.udma_busy(Bank::Rx) {
                // TODO: figure out why this timeout detection code is necessary.
                // It seems that some traffic during the UDMA access can cause the UDMA
                // engine to hang. For example, if we put a dcache_flush() routine in this
                // loop, it will fail immediately. This might be something to look into
                // in simulation.
                timeout += 1;
                if (self.mode == SpimMode::Quad) && (timeout > chunk.len() * 10_000) {
                    success = false;
                    break;
                }
                #[cfg(feature = "std")]
                if _use_yield {
                    xous::yield_slice();
                }
            }
            self.mem_cs(false);
            chunk.copy_from_slice(&self.rx_buf()[..chunk.len()]);
            offset += chunk.len();
        }
        success
    }

    /// This should only be called on SPI RAM -- not valid for FLASH devices, they need a programming routine!
    /// Note that `use_yield` is disallowed in interrupt contexts
    pub fn mem_ram_write(&mut self, addr: u32, buf: &[u8], _use_yield: bool) {
        // divide into buffer-sized chunks + repeat cycle on each buffer increment
        // this is because the size of the buffer is meant to represent the limit of the
        // target device's memory page (i.e., the point at which you'd wrap when reading)
        let mut offset = 0;
        for chunk in buf.chunks(self.tx_buf_len_bytes) {
            let chunk_addr = addr as usize + offset;
            let cmd_list = [
                SpimCmd::SendCmd(self.mode, 8, 0x38),
                SpimCmd::TxData(self.mode, SpimWordsPerXfer::Words1, 8 as u8, SpimEndian::MsbFirst, 3),
            ];
            let a = chunk_addr.to_be_bytes();
            self.tx_buf_mut()[..3].copy_from_slice(&[a[1], a[2], a[3]]);
            self.mem_cs(true);
            self.send_cmd_list(&cmd_list);
            // safety: this is safe because tx_buf_phys() slice is only used as a base/bounds reference
            unsafe { self.udma_enqueue(Bank::Tx, &self.tx_buf_phys::<u8>()[..3], CFG_EN | CFG_SIZE_8) }
            let wr_cmd = [SpimCmd::TxData(
                self.mode,
                SpimWordsPerXfer::Words1,
                8,
                SpimEndian::MsbFirst,
                chunk.len() as u32,
            )];
            while self.udma_busy(Bank::Tx) {
                #[cfg(feature = "std")]
                if _use_yield {
                    xous::yield_slice();
                }
            }
            self.send_cmd_list(&wr_cmd);
            self.tx_buf_mut()[..chunk.len()].copy_from_slice(chunk);
            // safety: this is safe because tx_buf_phys() slice is only used as a base/bounds reference
            unsafe {
                self.udma_enqueue(Bank::Tx, &self.tx_buf_phys::<u8>()[..chunk.len()], CFG_EN | CFG_SIZE_8)
            };
            while self.udma_busy(Bank::Tx) {
                #[cfg(feature = "std")]
                if _use_yield {
                    xous::yield_slice();
                }
            }
            self.mem_cs(false);
            offset += chunk.len();
        }
    }
}
