#!/bin/bash

ESP_HAL_CONFIG_PSRAM_MODE=octal cargo run --release --target=xtensa-esp32s3-none-elf --features="esp32s3,scd30"
