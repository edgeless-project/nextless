#!/bin/bash

cargo run --release --target=xtensa-esp32-none-elf --features="esp32,scd30,epaper_2_13"
