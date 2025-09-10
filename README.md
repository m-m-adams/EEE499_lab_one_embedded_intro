# Starter template for EE499

Stage 1 - Setup:
1. Install probe-rs `cargo install probe-rs-tools`
2. Install the board target `rustup target add thumbv6m-none-eabi`
3. Flash one pico with the probe firmware github.com/raspberrypi/debugprobe
4. Wire the debug header of probe 2 to the picoprobe pins 4 and 5. The black wire goes to ground
5. Wire the UART connections of the two picos together
3. `cargo run` will flash and run the program
4. `cargo embed` will flash and run the program with GDB. To use GDB in RustRover you need to download at least version 2025.2, otherwise you can use it from the terminal or in CLion with the Rust plugin

Stage 2 - Hello World!:
1. Add serial printing hello world to the main program. To do this you will use the log macro from defmt
2. Blink the onboard LED on and off. This LED is GPIO 0 controlled by the wifi chip
3. Every second toggle the LED and print ON or OFF to the terminal

Stage 3 - State Machines:
1. Connect the buttons and LEDs to your GPIO pins
2. Each LED should become a state machine controlled by the associated button
3. Your possible states are :
   1. Fade in and out over 3 seconds
   2. Blink every two seconds
   3. Toggle on button press
7. Transitions are:
   4. While the LED is toggled ON, long pressing the button switches to a fade
   5. While the LED is toggled ON, double pressing the button quickly switches to blinking
   6. While the LED is blinking or fading, pressing the button switches back to toggle mode in the ON state
   7. While the LED is blinking or fading, holding the button switches to toggle mode in the off state

Requirements:
   1. All four button/LED pairs should be their own state machine
   2. The states should be enum variants which all implement an `LED_state` trait. Use the `enum dispatch` crate to minimize boiler plate
   3. The event handler and update functions should be asynchronous
   4. The code should be expandable to any number of buttons
   5. Transitions between unlinked states should not be possible - one way to check this is using the "type state" design pattern