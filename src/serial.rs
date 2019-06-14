//! Serial interface
//!
//! You can use the `Serial` interface with these UART instances
//!
//! # UART0
//! - TX: Pin 17 IOF0
//! - RX: Pin 16 IOF0
//! - Interrupt::UART0
//!
//! # UART1
//! *Warning:* UART1 pins are not connected to package in FE310-G000
//! - TX: Pin 25 IOF0
//! - RX: Pin 24 IOF0
//! - Interrupt::UART1

use core::marker::PhantomData;
use core::convert::Infallible;

use embedded_hal::serial;
use nb;

#[allow(unused_imports)]
use e310x::{UART0, UART1};
use crate::clock::Clocks;
use crate::gpio::{IOF0, gpio0};
use crate::time::Bps;

// FIXME these should be "closed" traits
/// TX pin - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait TxPin<UART> {}

/// RX pin - DO NOT IMPLEMENT THIS TRAIT
pub unsafe trait RxPin<UART> {}

unsafe impl<T> TxPin<UART0> for gpio0::Pin17<IOF0<T>> {}
unsafe impl<T> RxPin<UART0> for gpio0::Pin16<IOF0<T>> {}

#[cfg(feature = "g002")]
mod g002_ims {
    use super::{TxPin, RxPin, UART1, gpio0, IOF0};
    unsafe impl<T> TxPin<UART1> for gpio0::Pin18<IOF0<T>> {}
    unsafe impl<T> RxPin<UART1> for gpio0::Pin23<IOF0<T>> {}
}

/// Serial abstraction
pub struct Serial<UART, PINS> {
    uart: UART,
    pins: PINS,
}

/// Serial receiver
pub struct Rx<UART> {
    _uart: PhantomData<UART>,
}

/// Serial transmitter
pub struct Tx<UART> {
    _uart: PhantomData<UART>,
}

macro_rules! hal {
    ($(
        $UARTX:ident: $uartX:ident
    )+) => {
        $(
            impl<TX, RX> Serial<$UARTX, (TX, RX)> {
                /// Configures a UART peripheral to provide serial communication
                pub fn $uartX(
                    uart: $UARTX,
                    pins: (TX, RX),
                    baud_rate: Bps,
                    clocks: Clocks,
                ) -> Self
                where
                    TX: TxPin<$UARTX>,
                    RX: RxPin<$UARTX>,
                {
                    let div = clocks.tlclk().0 / baud_rate.0 - 1;
                    unsafe { uart.div.write(|w| w.bits(div)); }

                    uart.txctrl.write(|w| w.enable().bit(true));
                    uart.rxctrl.write(|w| w.enable().bit(true));

                    Serial { uart, pins }
                }

                /// Starts listening for an interrupt event
                pub fn listen(self) -> Self {
                    self.uart.ie.write(|w| w.txwm().bit(false)
                                       .rxwm().bit(true));
                    self
                }

                /// Stops listening for an interrupt event
                pub fn unlisten(self) -> Self {
                    self.uart.ie.write(|w| w.txwm().bit(false)
                                       .rxwm().bit(false));
                    self
                }

                /// Splits the `Serial` abstraction into a transmitter and a
                /// receiver half
                pub fn split(self) -> (Tx<$UARTX>, Rx<$UARTX>) {
                    (
                        Tx {
                            _uart: PhantomData,
                        },
                        Rx {
                            _uart: PhantomData,
                        },
                    )
                }

                /// Releases the UART peripheral and associated pins
                pub fn free(self) -> ($UARTX, (TX, RX)) {
                    (self.uart, self.pins)
                }
            }

            impl serial::Read<u8> for Rx<$UARTX> {
                type Error = Infallible;

                fn read(&mut self) -> nb::Result<u8, Infallible> {
                    // NOTE(unsafe) atomic read with no side effects
                    let rxdata = unsafe { (*$UARTX::ptr()).rxdata.read() };

                    if rxdata.empty().bit_is_set() {
                        Err(::nb::Error::WouldBlock)
                    } else {
                        Ok(rxdata.data().bits() as u8)
                    }
                }
            }

            impl serial::Write<u8> for Tx<$UARTX> {
                type Error = Infallible;

                fn flush(&mut self) -> nb::Result<(), Infallible> {
                    // NOTE(unsafe) atomic read with no side effects
                    let txdata = unsafe { (*$UARTX::ptr()).txdata.read() };

                    if txdata.full().bit_is_set() {
                        Err(nb::Error::WouldBlock)
                    } else {
                        Ok(())
                    }
                }

                fn write(&mut self, byte: u8) -> nb::Result<(), Infallible> {
                    // NOTE(unsafe) atomic read with no side effects
                    let txdata = unsafe { (*$UARTX::ptr()).txdata.read() };

                    if txdata.full().bit_is_set() {
                        Err(::nb::Error::WouldBlock)
                    } else {
                        unsafe {
                            (*$UARTX::ptr()).txdata
                                .write(|w| w.data().bits(byte));
                        }
                        Ok(())
                    }
                }
            }
        )+
    }
}

hal! {
    UART0: uart0
}

#[cfg(feature = "g002")]
hal! {
    UART1: uart1
}
