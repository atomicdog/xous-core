#!/bin/bash

declare -a Crates=(
    "kernel"
    "loader"
    "gam"
    "status"
    "shellchat"
    "ime-frontend"
    "ime-plugin-shell"
    "graphics-server"
    "ticktimer-server"
    "log-server"
    "com"
    "xous-names"
    "keyboard"
    "trng"
    "llio"
    "susres"
    "codec"
    "sha2"
    "engine-25519"
    "spinor"
    "root-keys"
    "jtag"
    "net"
    "dns"
    "pddb"
    "modals"
    "usb-device-xous"
    "vault"
    "ball"
    "repl"
)

if [ -e "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi
RISCV_TOOLS=/opt/riscv64-unknown-elf
export PATH=$RISCV_TOOLS/bin:$PATH

env | sort > env.txt

#cargo clean
mkdir -p reports
mkdir -p reports/old
# move just the files, not the directory
find reports/ -maxdepth 1 -type f -name '[!.]*' -exec mv -n {} reports/old/ \;

echo "===== TURNING OF STRIP ====="
sed -i 's/strip = true/strip = false/g' Cargo.toml

echo "===== STARTING BUILD at $(date) ====="
cargo xtask app-image ball repl vault

echo "===== ANALYZING at $(date) ====="
for val in ${Crates[@]}; do
    # dump the header summary
    riscv64-unknown-elf-objdump -h target/riscv32imac-unknown-xous-elf/release/$val > reports/$val.txt
    # dump the sorted list of objects
    riscv64-unknown-elf-nm -r --size-sort --print-size target/riscv32imac-unknown-xous-elf/release/$val | rustfilt >> reports/$val.txt
    # dump the disassembly
    riscv64-unknown-elf-objdump -S -d target/riscv32imac-unknown-xous-elf/release/$val | rustfilt > reports/$val.list
done


echo "===== REVERTING STRIP ====="
git checkout Cargo.toml