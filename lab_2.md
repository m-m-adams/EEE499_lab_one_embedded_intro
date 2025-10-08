# Starter template for EE499

# Objectives
Interrupts are useful as they allow us to momentarily suspend the execution of the main program,
go perform some action(s) within an Interrupt Service Routine (ISR) and resume execution without losing clock cycles.
This lab is an introduction to using timers to control the flow of a real-time application with multiple tasks.
In the first part of the lab, you will write an ISR that uses an interrupt to perform some actions.
In this first part of the lab you will be toggling the LEDs on your board on and off.

In the second part of the lab you will learn how to use interrupts to drive an embassy executor

In the third part you will implement different scheduling mechanisms based on the policy that is given to you in the lab instructions.

# Part 1
The RP2040 features a 64 bit timer (used by embassy-time), a 24 bit sys tick counter, and 8 16 bit timers used for PWM.
We'll use the PWM timers to output an interrupt that toggles the LEDs on our board.

Each PWM block is a counter that counts up to a specified value and then resets to zero.
The PWM block can be configured to generate an interrupt when the counter reaches a specified value.
The frequency of that interrupt is controlled by the system clock frequency, the clock divider, and the top value.
The system clock frequency is 133 MHz by default. The period of a timer can be calculated in ticks as follows:
```aiignore
(top + 1) * (phase_correct ? 1 : 2) * divider
```
and that can be converted to seconds by dividing by the system clock frequency. Equivalently the required top value can be calculated as:
```aiignore
top = (period * system_clock_frequency) / ((phase_correct ? 1 : 2) * divider) - 1
```
As the divider is limited (u8.4 fixed point) you'd typically get that into the right ballpark and then tweak with top
To get a period of 1 second we'll need a divider of 2048. That will give us a top value of 64941.
Unfortunately the divider value in hardware is limited to 255. Instead we'll need to generate interrupts at a faster rate
and divide down further in software.

The PWM timer can be configured to generate an interrupt when the counter wraps (passes the top value and resets to zero).
Unfortunately there are 8 PWMs sharing the same interrupt.
We'll need to check the PWM interrupt status register to determine which one triggered. This register is a bitfield with one bit per PWM.
As multiple interrupts can trigger at once if the PWM rates are synced up we need to check all of them (e.g. slice 1 triggering doesn't mean slice 2 didn't)

Determine a combination of settings to toggle the 4 LEDs at the rates of 1 Hz, 3 Hz, 5 Hz, and 7 Hz using the PWM timers.
Leave a comment showing your math on each config.
Commit your code and push it to github. Please make sure it builds.

## Part 2 - Clock based scheduling
You'll now use clock based scheduling to implement a simple real-time application.
Make a schedule for the following tasks:
τ1= (4,1), τ2= (5,1.5), τ3 = (20,1) , τ4 = (20,2)
Mock each task by blocking for the appropriate amount of time with `embassy_time::block_for(Duration::from_millis(appropriate amount of time));`
This just blocks for that length of time using the system timer. 

Implement a non pre emptive clock driven scheduler to run these tasks. 
This means that once a task is given the processor it runs to completion. 
We will assume that no blocking for resources is possible. 
A task will only block when it is waiting for its next execution.
While each task is running it should light an LED (task 1 lights LED 1, task 2 lights LED 2, etc.)

You are to implement the following:

1. A TCB struct (or struct of structs) that includes a task's priority, period, max execution time, run function, and remaining tics.
2. A ready and a blocked "queue" 
    1. Just conceptually queues, for the implementation ready is probably a channel and blocked is probably an array of N Option<TCB>s
3. A sleep() function that the tasks can call to delay their next release.
    1. Each time a task finishes it sets its a number of ticks (50ms each) to delay and places itself in the blocked queue. 
    2. You'll probably do this by implementing a `sleep(thread: TCB)` function that sends to a channel, and then reading from that channel in the ISR
4. A clock tick ISR that runs every 50ms based on a PWM timer. 
    1. Each time the clock tick ISR runs, it inspects the blocked queue and updates the next release time of each task by decrementing the remaining ticks.
    2. If the number of remaining ticks is 0 after the update, the ISR moves the node to the ready queue.
    3. When all the nodes of the blocked queue have been updated and the appropriate tasks have been moved back to the ready queue the ISR calls the scheduler.
5. A priority-based scheduler that runs every clock tick, as it is called by the clock tic ISR.
    1. the scheduler is to inspect the blocked queue and sends tasks to the channel in priority order
    2. If a task is ready to be ran, it is placed in the ready queue and the scheduler pends IRQ 0
6. A receiver in IRQ 0 that runs tasks as they are sent

Your scheduler must be generic enough that if a new set of tasks was given, it could work

Commit your code and push it to github. Please make sure it builds.

## Part 3 - Pre emptive scheduling with Embassy
The goal is to run the clock driven schedule from part 2 in parallel with the async schedule from lab 1. The starter code for this lab
includes a working solution of lab1 for reference. 

To run preemptively we'll use two different executors, one thread mode, and one interrupt mode using IRQ 0. Determine the priorities for
the button/LED tasks and the mocked real time tasks using earliest-deadline-first scheduling, and put the higher priority tasks
in the interrupt executor. If you determine that you need more priorities you can add more interrupt executors. Use the 
SWI interrupts for the interrupt executor. They work by triggering a software interrupt when a future is ready to be 
polled. In your real time tasks your PWM interrupt will send a task to a channel in an interrupt task. 
That channel will then trigger SWI0. When your ISR ends rather than returning to SVC mode it will enter the SWI0 handler
which polls the future and runs the task.
```aiignore
static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();

#[interrupt]
unsafe fn SWI_IRQ_0() {
    unsafe { EXECUTOR_HIGH.on_interrupt() }
}

... in main
    // set interrupt priority, 0 is highest but should be kept for hardware interrupts. Otherwise they can't trigger to 
    // set tasks as ready
    interrupt::SWI_IRQ_0.set_priority(Priority::P2);
    let spawner = EXECUTOR_HIGH.start(interrupt::SWI_IRQ_0);
    spawner.spawn(unwrap!(run_med()));

```

Use 2 LEDs in state machines as Lab 1, and use the other two LEDs to show which task is running in the real time schedule.
Real time task 1 should light LED 1, real time task 2 should light LED 2, task 3 blinks LED 1, task 4 blinks LED 2.


Once again commit your code and push it to github. Please make sure it builds.