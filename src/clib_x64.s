.text
.globl init_stack
.type init_stack,@function
.align 16
init_stack:
    movq %rdi, %rax
    leaq -0x40(%rax), %rax
    movq %rsi, 0x30(%rax)
    leaq quit(%rip), %rcx
    movq %rcx, 0x38(%rax) 
    ret
quit:
    xorq %rdi, %rdi
    
    movq $60, %rax
    syscall

.size init_stack,.-init_stack
.section .note.GNU-stack,"",%progbits

.text
.globl switch_stack
.type switch_stack,@function
.align 16
switch_stack:
    pushq  %rbp
    pushq  %rbx
    pushq  %r15
    pushq  %r14
    pushq  %r13
    pushq  %r12

    movq  %rsp, (%rdi)
    movq  %rsi, %rsp

    popq  %r12
    popq  %r13
    popq  %r14
    popq  %r15
    popq  %rbx
    popq  %rbp

    popq  %r8 

    /* Context pointer of ctx_function first argument */
    movq  %rdx, %rdi

    jmp  *%r8
.size switch_stack,.-switch_stack
.section .note.GNU-stack,"",%progbits

.text
.globl registers_get
.type registers_get,@function
.align 16
registers_get:
    movq %rax,0(%rdi)
    movq %rbx,8(%rdi)
    movq %rcx,16(%rdi)
    movq %rdx,24(%rdi)
    movq %rsi,32(%rdi)
    movq %rsp,40(%rdi)
    movq %rbp,48(%rdi)
    movq %r8, 56(%rdi)
    movq %r9, 64(%rdi)
    movq %r10,72(%rdi)
    movq %r11,80(%rdi)
    movq %r12,88(%rdi)
    movq %r13,96(%rdi)
    movq %r14,104(%rdi)
    movq %r15,112(%rdi)

    ret
.size registers_get,.-registers_get
.section .note.GNU-stack,"",%progbits

.text
.globl get_stackptr
.type get_stackptr,@function
.align 16
get_stackptr:
    movq %rsp,%rax

    ret
.size get_stackptr,.-get_stackptr
.section .note.GNU-stack,"",%progbits