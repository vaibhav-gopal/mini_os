
# MiniOS, a small operating system that can run on most x86_64 platforms

> Built entirely on Rust --> Based on the blogOS tutorials/blog by Phillip Oppermann

> Run using 'cargo run'

- Opens a QEMU instance and runs the OS on that

note: I did not write/set up the bootloader but it does a lot of things:
- Set up protected mode
- Set up the stack and guard pages
- Set up a 4-level basic paging layout for the kernel (remapped later during kernel executation)
- others...
