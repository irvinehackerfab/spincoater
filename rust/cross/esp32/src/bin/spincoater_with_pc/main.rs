#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Event, Input, InputConfig, Io, Level, Output, OutputConfig, Pull},
    interrupt::software::SoftwareInterruptControl,
    mcpwm::{McPwm, PeripheralClockConfig, operator::PwmPinConfig, timer::PwmWorkingMode},
    timer::timg::TimerGroup,
    uart::Uart,
};
use esp_println::println;
use esp32::{
    REQUEST_CHANNEL, REQUEST_RESPONSE_SIGNAL, SECOND_CORE_STACK,
    gpio::{
        encoder::ENCODER,
        interrupt_handler,
        pwm::{FREQUENCY, PERIOD, PERIPHERAL_CLOCK_PRESCALER, SETPOINTS},
    },
    rpc::{Context, Dispatcher, FRAME_BUFFER, WIRE_STORAGE},
    runners::motion_profile::{Runner, run},
};
use postcard_rpc::server::{Dispatch, Server};
use sc_messages::{icd::BAUD_RATE, pwm::STOP_DUTY};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "main is the only place you should be allowed to allocate large buffers."
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO0
    // - GPIO2
    // - GPIO5
    // - GPIO12
    // - GPIO15
    // These GPIO pins are in use by some feature of the module and should not be used.
    // let _ = peripherals.GPIO6;
    // let _ = peripherals.GPIO7;
    // let _ = peripherals.GPIO8;
    // let _ = peripherals.GPIO9;
    // let _ = peripherals.GPIO10;
    // let _ = peripherals.GPIO11;

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    // Run the encoder task/ISR on the second core so it doesn't block the program.
    let software_interrupts = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start_second_core(
        peripherals.CPU_CTRL,
        software_interrupts.software_interrupt0,
        software_interrupts.software_interrupt1,
        SECOND_CORE_STACK.take(),
        || {
            // Set the interrupt handler for GPIO.
            let mut io = Io::new(peripherals.IO_MUX);
            io.set_interrupt_handler(interrupt_handler);

            // Initialize encoder pin
            let mut encoder = Input::new(
                peripherals.GPIO27,
                InputConfig::default().with_pull(Pull::Down),
            );

            // Start listening for rising edges
            ENCODER.with(|encoder_memory_cell| {
                encoder.listen(Event::RisingEdge);
                encoder_memory_cell.replace(encoder);
            });
        },
    );

    // Initialize PWM
    let clock_cfg = PeripheralClockConfig::with_prescaler(PERIPHERAL_CLOCK_PRESCALER);
    let mut mcpwm = McPwm::new(peripherals.MCPWM0, clock_cfg);
    mcpwm.operator0.set_timer(&mcpwm.timer0);
    // connect operator0 to pin IO26:
    // https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#j3
    let mut pwm_pin = mcpwm
        .operator0
        .with_pin_a(peripherals.GPIO26, PwmPinConfig::UP_ACTIVE_HIGH);
    // start timer with timestamp values in the range that we choose.
    let timer_clock_cfg = clock_cfg
        .timer_clock_with_frequency(PERIOD, PwmWorkingMode::Increase, FREQUENCY)
        .expect("Failed to create TimerClockConfig");
    mcpwm.timer0.start(timer_clock_cfg);
    pwm_pin.set_timestamp(STOP_DUTY);

    // Initialize vacuum pump pin
    let vacuum_pump_pin = Output::new(peripherals.GPIO17, Level::Low, OutputConfig::default());

    // Setup communication between tasks
    let request_channel = REQUEST_CHANNEL.take();

    // Initialize the setpoint list with a starting setpoint of (0, 0).
    let setpoints = SETPOINTS.take();

    let server_signal = REQUEST_RESPONSE_SIGNAL.take();

    // Setup context
    let context = Context::new(request_channel.sender(), server_signal, vacuum_pump_pin);

    // Setup UART and postcard-rpc after we're done with the spawner
    let config = esp_hal::uart::Config::default().with_baudrate(BAUD_RATE);
    // Select pins based on the cargo feature
    cfg_select! {
        feature = "uart_over_adapter" => {
            let uart = Uart::new(peripherals.UART1, config)
                .expect("Failed to initialize UART")
                .with_tx(peripherals.GPIO23)
                .with_rx(peripherals.GPIO22)
                .into_async();
        }
        _ => {
            println!("Taking control of the UART port. Please close RTT and open the host PC program.");
            // We have to wait for the print statement to arrive at `espflash`'s RTT monitor before taking control.
            Timer::after_millis(100).await;
            let uart = Uart::new(peripherals.UART1, config)
                .expect("Failed to initialize UART")
                .with_tx(peripherals.GPIO1)
                .with_rx(peripherals.GPIO3)
                .into_async();
        }
    }
    let (rx, tx) = uart.split();
    let dispatcher = Dispatcher::new(context, ());
    let (wire_rx, wire_tx) = WIRE_STORAGE
        .init(rx, tx)
        .expect("Failed to create wire RX and TX");
    let frame_buffer = FRAME_BUFFER.take();
    let vkk = dispatcher.min_key_len();
    let mut server = Server::new(
        wire_tx,
        wire_rx,
        frame_buffer.as_mut_slice(),
        dispatcher,
        vkk,
    );

    let runner = Runner::new(
        setpoints,
        pwm_pin,
        request_channel.receiver(),
        server.sender(),
        server_signal,
    );
    spawner.must_spawn(run(runner));

    loop {
        let _ = server.run().await;
    }
}
