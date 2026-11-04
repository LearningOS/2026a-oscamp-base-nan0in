//! # Stackful Coroutine and Context Switch (riscv64)
//!
//! In this exercise, you implement the minimal context switch using inline assembly,
//! which is the core mechanism of OS thread scheduling. This crate is **riscv64 only**;
//! run `cargo test` on riscv64 Linux, or use the repo's normal flow (`./check.sh` / `oscamp`) on x86 with QEMU.
//!
//! ## Key Concepts
//! - **Callee-saved registers**: Save and restore them on switch so the switched-away task can resume correctly later.
//! - **Stack pointer `sp`** and **return address `ra`**: Restore them in the new context; the first time we switch to a task, `ret` jumps to `ra` (the entry point).
//! - Inline assembly: `core::arch::asm!`
//!
//! ## riscv64 ABI (for this exercise)
//! - Callee-saved: `sp`, `ra`, `s0`–`s11`. The `ret` instruction is `jalr zero, 0(ra)`.
//! - First and second arguments: `a0` (old context), `a1` (new context).

#![cfg(target_arch = "riscv64")]

use core::arch::naked_asm;

/// Saved register state for one task (riscv64). Layout must match the offsets used in the asm below: for one task (riscv64). Layout must match the offsets used in the asm below:
/// `sp` at 0, `ra` at 8, then `s0`–`s11` at 16, 24, … 104.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TaskContext {
    pub sp: u64,
    pub ra: u64,
    pub s0: u64,
    pub s1: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
}

impl TaskContext {
    pub const fn empty() -> Self {
        Self {
            sp: 0,
            ra: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }

    /// Initialize this context so that when we switch to it, execution starts at `entry`.
    ///
    /// - Set `ra = entry` so that the first `ret` in the new context jumps to `entry`.
    /// - Set `sp = stack_top` with 16-byte alignment (RISC-V ABI requires 16-byte aligned stack at function entry).
    /// - Leave `s0`–`s11` zero; they will be loaded on switch.
    pub fn init(&mut self, stack_top: usize, entry: usize) {
        // ra = entry：第一次切到这个上下文时，switch_context 末尾的 ret
        // 就是「跳到 ra」，于是直接从 entry 开始执行。
        self.ra = entry as u64;

        // RISC-V ABI 要求函数入口处 sp 16 字节对齐（栈向下生长）。
        // 向下取整而不是向上，避免越过 stack_top 踩到别人的内存。
        self.sp = (stack_top & !15) as u64;

        // s0–s11 保持 0：它们切过去时才从新上下文载入，现在填什么都无所谓。
    }
}

/// Switch from `old` to `new` context: save current callee-saved regs into `old`, load from `new`, then `ret` (jumps to `new.ra`).
///
/// In asm: store `sp`, `ra`, `s0`–`s11` to `[a0]` (old), load from `[a1]` (new), zero `a0`/`a1` so we do not leak pointers into the new context, then `ret`.
///
/// Must be `#[unsafe(naked)]` to prevent the compiler from generating a prologue/epilogue.
#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(_old: &mut TaskContext, _new: &TaskContext) {
    // naked 函数不生成序言/尾声，函数体就是这几条汇编：
    //   a0 = old 指针，a1 = new 指针（riscv64 C ABI 的前两个参数）
    naked_asm!(
        // ---- 保存当前（old）的被调用者保存寄存器 ----
        "sd sp,   0(a0)",
        "sd ra,   8(a0)",
        "sd s0,  16(a0)",
        "sd s1,  24(a0)",
        "sd s2,  32(a0)",
        "sd s3,  40(a0)",
        "sd s4,  48(a0)",
        "sd s5,  56(a0)",
        "sd s6,  64(a0)",
        "sd s7,  72(a0)",
        "sd s8,  80(a0)",
        "sd s9,  88(a0)",
        "sd s10, 96(a0)",
        "sd s11,104(a0)",
        // ---- 从 new 上下文恢复 ----
        "ld sp,   0(a1)",
        "ld ra,   8(a1)",
        "ld s0,  16(a1)",
        "ld s1,  24(a1)",
        "ld s2,  32(a1)",
        "ld s3,  40(a1)",
        "ld s4,  48(a1)",
        "ld s5,  56(a1)",
        "ld s6,  64(a1)",
        "ld s7,  72(a1)",
        "ld s8,  80(a1)",
        "ld s9,  88(a1)",
        "ld s10, 96(a1)",
        "ld s11,104(a1)",
        // 把 a0/a1 清零，别把旧地址泄漏进新任务。
        "li a0, 0",
        "li a1, 0",
        // ret == jalr zero, 0(ra)：跳到新上下文的 ra。
        // 第一次切到新任务时，ra 就是 entry，于是任务从 entry 开始跑。
        "ret",
    );
}

const STACK_SIZE: usize = 1024 * 64;

/// Allocate a stack for a coroutine. Returns `(buffer, stack_top)` where `stack_top` is the high address
/// (stack grows down). The buffer must be kept alive for the lifetime of the context using this stack.
pub fn alloc_stack() -> (Vec<u8>, usize) {
    // Vec 在堆上分配一块连续内存，把它整块当作协程栈。
    // 用 vec![0u8; N] 而不是 Vec::with_capacity，确保内存真的被初始化过。
    let buf = vec![0u8; STACK_SIZE];

    // 栈向下生长，所以栈顶 = 起始地址 + 大小。
    // glibc 的 malloc 对 ≥16 字节的块保证 16 字节对齐，
    // 而 STACK_SIZE 也是 16 的倍数，所以栈顶天然满足 ABI 要求；
    // 真正的对齐处理交给 TaskContext::init 再做一次兜底。
    let top = buf.as_ptr() as usize + STACK_SIZE;

    // buf 必须由调用者持有：一旦 drop，栈内存就被回收了，
    // 某个协程再切回来就会踩到已释放的内存。
    (buf, top)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    extern "C" fn task_entry() {
        COUNTER.store(42, Ordering::SeqCst);
        loop {
            std::hint::spin_loop();
        }
    }

    #[test]
    fn test_alloc_stack() {
        let (buf, top) = alloc_stack();
        assert_eq!(top, buf.as_ptr() as usize + STACK_SIZE);
        assert!(top % 16 == 0);
    }

    #[test]
    fn test_context_init() {
        let (buf, top) = alloc_stack();
        let _ = buf;
        let mut ctx = TaskContext::empty();
        let entry = task_entry as *const () as usize;
        ctx.init(top, entry);
        assert_eq!(ctx.ra, entry as u64);
        assert!(ctx.sp != 0);
    }

    #[test]
    fn test_switch_to_task() {
        COUNTER.store(0, Ordering::SeqCst);

        static mut MAIN_CTX_PTR: *mut TaskContext = std::ptr::null_mut();
        static mut TASK_CTX_PTR: *mut TaskContext = std::ptr::null_mut();

        extern "C" fn cooperative_task() {
            COUNTER.store(99, Ordering::SeqCst);
            unsafe {
                switch_context(&mut *TASK_CTX_PTR, &*MAIN_CTX_PTR);
            }
        }

        let (_stack_buf, stack_top) = alloc_stack();
        let mut main_ctx = TaskContext::empty();
        let mut task_ctx = TaskContext::empty();
        task_ctx.init(stack_top, cooperative_task as *const () as usize);

        unsafe {
            MAIN_CTX_PTR = &mut main_ctx;
            TASK_CTX_PTR = &mut task_ctx;
            switch_context(&mut main_ctx, &task_ctx);
        }

        assert_eq!(COUNTER.load(Ordering::SeqCst), 99);
    }
}
