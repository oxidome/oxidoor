// Copyright 2025 AprilNEA LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

use embassy_executor::Executor;
use esp_idf_svc::hal::{
    gpio::{PinDriver, Pull},
    prelude::Peripherals,
};

mod lock;

mod task;
use static_cell::StaticCell;
use task::door_control;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    //     // log::info!("Hello, world!");
    let peripherals = Peripherals::take().expect("Failed to take peripherals");

    // Configuration relay output (door lock control)
    let mut door_lock =
        PinDriver::output(peripherals.pins.gpio5).expect("Failed to create door lock");

    // Initial state: Locked
    door_lock.set_high().unwrap();

    //Configuration switch input (internally pulled up, low level when pressed)
    let mut button1 = PinDriver::input(peripherals.pins.gpio6).expect("Failed to create button 1");
    button1.set_pull(Pull::Up).unwrap();

    let mut button2 = PinDriver::input(peripherals.pins.gpio7).expect("Failed to create button 2");
    button2.set_pull(Pull::Up).unwrap();

    // Create and run the Embassy Executor
    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| {
        spawner
            .spawn(door_control(door_lock, button1, button2))
            .ok();
    });
}
