#![no_std]
#![no_main]

extern crate panic_halt as _;


extern crate stm32f1xx_hal as hal;


use crate::hal::{
    gpio::*,
    prelude::*,
    stm32::{interrupt, Interrupt, Peripherals, TIM2},
    timer::*,
};

use core::cell::RefCell;
use cortex_m::{
    asm::wfi,
    interrupt::Mutex,
    peripheral::Peripherals as c_m_Peripherals,
};
use cortex_m_rt::entry;

type LEDPIN = gpioc::PC13<Output<PushPull>>;

static G_LED: Mutex<RefCell<Option<LEDPIN>>> = Mutex::new(RefCell::new(None));
static G_TIM: Mutex<RefCell<Option<CountDownTimer<TIM2>>>> = Mutex::new(RefCell::new(None));

#[interrupt]
fn TIM2() {
    static mut LED: Option<LEDPIN> = None;
    static mut TIM: Option<CountDownTimer<TIM2>> = None;

    let led = LED.get_or_insert_with(|| {
        cortex_m::interrupt::free(|cs| {
            G_LED.borrow(cs).replace(None).unwrap()
        })
    });

    let tim = TIM.get_or_insert_with(|| {
        cortex_m::interrupt::free(|cs| {
            // Move LED pin here, leaving a None in its place
            G_TIM.borrow(cs).replace(None).unwrap()
        })
    });

    led.toggle().ok();
    tim.wait().ok();
}

#[entry]
fn main() -> ! {
    if let (Some(dp), Some(cp)) = (Peripherals::take(), c_m_Peripherals::take()) {
        cortex_m::interrupt::free(move |cs| {
            let mut rcc = dp.RCC.constrain();
            let mut flash = dp.FLASH.constrain();
            let clocks = rcc
                .cfgr
                .sysclk(8.mhz())
                .pclk1(8.mhz())
                .freeze(&mut flash.acr);

            let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);
            let led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

            *G_LED.borrow(cs).borrow_mut() = Some(led);

            let mut timer = Timer::tim2(dp.TIM2, &clocks, &mut rcc.apb1).start_count_down(10.hz());

            timer.listen(Event::Update);

            *G_TIM.borrow(cs).borrow_mut() = Some(timer);

            let mut nvic = cp.NVIC;

            unsafe {
                nvic.set_priority(Interrupt::TIM2, 1);
                cortex_m::peripheral::NVIC::unmask(Interrupt::TIM2);
            }

            cortex_m::peripheral::NVIC::unpend(Interrupt::TIM2);
        });
    }

    loop {
        wfi();
    }
}