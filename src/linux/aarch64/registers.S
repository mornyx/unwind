    .text
    .globl unwind_init_registers
unwind_init_registers:
    stp    x0, x1,  [x0, #0x000]
    stp    x2, x3,  [x0, #0x010]
    stp    x4, x5,  [x0, #0x020]
    stp    x6, x7,  [x0, #0x030]
    stp    x8, x9,  [x0, #0x040]
    stp    x10,x11, [x0, #0x050]
    stp    x12,x13, [x0, #0x060]
    stp    x14,x15, [x0, #0x070]
    stp    x16,x17, [x0, #0x080]
    stp    x18,x19, [x0, #0x090]
    stp    x20,x21, [x0, #0x0A0]
    stp    x22,x23, [x0, #0x0B0]
    stp    x24,x25, [x0, #0x0C0]
    stp    x26,x27, [x0, #0x0D0]
    stp    x28,x29, [x0, #0x0E0]
    str    x30,     [x0, #0x0F0]
    mov    x1,sp
    str    x1,      [x0, #0x0F8]
    ret
