.intel_syntax noprefix
.section .text:
.globl __vdso_clock_gettime
.globl __vdso_getcpu
.globl __vdso_gettimeofday
.globl __vdso_time

__vdso_clock_gettime:
    mov rax, 228
    syscall
    ret

__vdso_getcpu:
    mov rax, 309
    syscall
    ret

__vdso_gettimeofday:
    mov rax, 96
    syscall
    ret

__vdso_time:
    mov rax, 201
    syscall
    ret
