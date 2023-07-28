## The integration tests in this sub-directory are treated as completely seperate environments and executables

Therefore compared the initialization routines that main.rs and lib.rs might have in their _start functions, the integration tests can carefully choose and select any feature SET out of the program with specific environments

**There are 3 places where tests can be**:
- The integration tests found here
- Unit tests in main.rs and lib.rs
- Unit tests in each of the modules (like vga_buffer.rs)