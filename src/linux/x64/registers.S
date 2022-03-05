    .text
    .globl unwind_init_registers
unwind_init_registers:
    movq    %rax,   (%rdi)
    movq    %rdx,  8(%rdi)
    movq    %rcx, 16(%rdi)
    movq    %rbx, 24(%rdi)
    movq    %rsi, 32(%rdi)
    movq    %rdi, 40(%rdi)
    movq    %rbp, 48(%rdi)
    movq    %rsp, 56(%rdi)
    addq    $8,   56(%rdi)
    movq    %r8,  64(%rdi)
    movq    %r9,  72(%rdi)
    movq    %r10, 80(%rdi)
    movq    %r11, 88(%rdi)
    movq    %r12, 96(%rdi)
    movq    %r13,104(%rdi)
    movq    %r14,112(%rdi)
    movq    %r15,120(%rdi)
    movq    (%rsp),%rsi
    movq    %rsi,128(%rdi)
    xorl    %eax, %eax
    ret
