# no_std 模块：完整实现、推导与验证过程

本文对应 `exercises/02_no_std_dev/` 下的五个练习。代码实现位于各题
`src/lib.rs`，新增回归测试位于各题 `tests/regressions.rs`，仅保存在本地。

## 本次修改位置与提交边界

原题的 **24 处 TODO 及其连续 Hint/Steps 注释原样保留**，实现放在对应注释下方。
这些 TODO 是题目定位标记，不代表尚未完成；原来的 `todo!()` 已替换为实际代码。
空闲链表题的 bump 分配采用同一把锁串行保护，因此不需要照提示重复做 CAS。

| 文件 | 为完成任务所做的修改 |
|---|---|
| `01_mem_primitives/src/lib.rs` | 实现五个内存/字符串函数；按地址选择 memmove 复制方向；补充指针安全约束。 |
| `02_bump_allocator/src/lib.rs` | 实现对齐、溢出与容量检查、CAS 重试；将 reset 收紧为 unsafe，并同步原有单元测试调用点。 |
| `03_free_list_allocator/src/lib.rs` | 实现 first-fit、摘链、尾块拆分、bump 后备与释放头插；用自旋锁和 RAII guard 保护完整操作，统一测试与生产路径。 |
| `04_syscall_wrapper/src/lib.rs` | 填充三架构 ABI、内联汇编与调用号，实现 read/write/close/exit；增加 RISC-V 执行分支并说明 exit 的线程语义。 |
| `05_fd_table/src/lib.rs` | 改用 no_std + alloc；实现 Vec 空槽表、最小 fd 复用、Arc 查询、关闭与计数。 |
| `README.md` | 记录各题推导、实现过程、安全约束、验证方法及修改清单。 |

新增的五个 `tests/` 目录通过 `.git/info/exclude` 中的精确目录规则排除，
保留在本地运行，不纳入常规 `git add`，也不修改共享的 `.gitignore`。
原有 `src/lib.rs` 内的单元测试保留；其中 reset 调用点的修改属于接口适配。
本次未执行提交或推送，其他模块已有的修改保持不变。

## 一、先明确 no_std 的边界

本模块的生产库使用：

```rust
#![cfg_attr(not(test), no_std)]
```

这表示普通构建时不自动引入 `std`，单元测试时仍允许测试框架使用它。
`no_std` 不是“禁止 unsafe”，也不是“禁止一切堆分配”：

- `core`：指针、切片、整数、原子类型、`Layout`、`GlobalAlloc` 等基础能力。
- `alloc`：`Vec`、`Arc` 等需要分配器、但本身不要求操作系统的类型。
- `std`：进一步提供线程、文件、网络、同步等宿主系统能力。

前四题的生产实现只使用 `core`；第五题用 `alloc::vec::Vec` 和
`alloc::sync::Arc`，不依赖生产环境的 `std`。
这几个练习仍是**库**，不是已经具备入口、panic handler 和完整启动过程的裸机内核。
最终裸机程序使用第五题时，还需要自行提供全局分配器等运行条件。
系统调用题则明确面向 Linux，`no_std` 与“运行在裸机上”是两个不同概念。

集成测试在独立测试程序中链接生产库，因此还会覆盖普通 `no_std` 编译路径，
不仅检查 `cfg(test)` 下的单元测试路径。

## 二、01_mem_primitives：逐字节建立内存操作

### 1. memcpy 与 memset

`my_memcpy` 的核心是：

```rust
for i in 0..n {
    unsafe { dst.add(i).write(src.add(i).read()) };
}
dst
```

`my_memset` 不读取源地址，只把 `c` 写入 `dst[i]`。
两者返回原始目标指针。长度为零时循环不执行。
实现没有调用 `core::ptr::copy` / `copy_nonoverlapping`，符合题目手写循环的要求。
编译器如何优化这些循环属于另一层问题；若以后实现同名 libc 符号，需要另外检查
优化后的产物，避免编译器把实现转换成对自身的调用。本题导出名是 `my_*`。

### 2. 为什么 memmove 要区分方向

假设将 `[1, 2, 3, 4, 5]` 的前四个字节右移一格。
若从前向后复制，刚写入的第二格会覆盖后续仍要读取的数据，结果会被连续污染。

```text
右移：从尾到头复制
  buf[4] = buf[3]
  buf[3] = buf[2]
  buf[2] = buf[1]
  buf[1] = buf[0]
结果：[1, 1, 2, 3, 4]
```

实现采用：`dst > src` 时反向复制，其余情况正向复制。
即使两段内存不重叠，这个方向选择仍然正确，并省去计算 `src + n` 的需要。
没有使用额外缓冲区，额外空间为 O(1)。

新增测试穷举八字节数组中所有合法的源起点、目标起点、复制长度，
与标准切片的 `copy_within` 逐项比较，包含左移、右移、原地复制和空区间。

### 3. strlen 与 strcmp

- `my_strlen` 找到第一个零字节即停止，返回值不包括零字节。
- `my_strcmp` 逐字节比较；首次不同就返回 `i32::from(a) - i32::from(b)`。
- 如果相同且为零，两者同时结束，差值就是零。
- 必须按 `u8` 处理字节，避免把 `0xff` 当成负字符。

`strlen` 计算的是字节长度，不是 Unicode 字符数。
这些函数是 `unsafe` 接口：源区间必须可读且初始化，目标区间必须可写；
`memcpy` 另要求不重叠，字符串函数要求在有效可读范围内存在结束零字节。
本题没有也不应通过盲目扫描来“验证任意地址是否安全”。

复杂度：复制、填充为 O(n)，字符串操作为 O(扫描字节数)。

## 三、02_bump_allocator：对齐、边界检查与 CAS

### 1. 状态与对齐公式

```text
heap_start                 next                  heap_end
|------ 已保留的空间 --------|------ 尚未使用 ---------|
```

`next` 是下一次保留空间的起点。设对齐要求为 `align`，向上对齐公式为：

```text
aligned = (next + align - 1) & !(align - 1)
end     = aligned + size
```

`Layout` 的对齐是非零的二次幂，因此 `align - 1` 的低若干位全为 1。
先加再清除这些低位，即得到不小于原地址的最小对齐地址。
例如 `next=0x1003, align=8`，得到 `aligned=0x1008`。

### 2. 为什么先 checked_add 再比较边界

普通整数加法在调试模式可能 panic，在优化模式可能回绕。
若地址回绕成一个很小的值，直接比较 `end <= heap_end` 会产生错误判断。
实现对“对齐加法”和“长度加法”都使用 `checked_add`，失败返回空指针。
`end == heap_end` 合法，只有 `end > heap_end` 才表示容量不足。

### 3. 为什么不能 load 后直接 store

两个线程可能同时读到相同的 `next`，进而返回同一段空间。
实现用 `compare_exchange` 提交一次保留：

```text
读取 next
  ↓
计算 aligned / end，检查算术和容量
  ↓
CAS(next, end)
  ├─ 成功：返回 aligned
  └─ 失败：取 CAS 返回的新 next，重新计算
```

只有成功的 CAS 才改变共享状态；失败分配不会消耗空间。
本题按提示使用 `SeqCst`。CAS 只负责空间保留，不负责自动发布对象初始化结果；
使用者跨线程共享对象时仍需自己的同步。

### 4. dealloc 与 reset 的安全区别

`dealloc` 保持空实现：单独释放一个块不会回退 `next`，否则后续块可能被覆盖。
`reset` 则一次释放整个区域。

本次将 `reset(&self)` 调整为 `unsafe fn reset(&self)`，同时更新原有调用点。
调用者必须保证旧分配已不再使用，且重置期间没有并发分配、释放。
这是刻意收紧的接口：原来的安全方法允许活对象仍存在时重置，后续分配就可能与其重叠。

新增测试覆盖：失败后仍可完整使用堆、极大请求被拒绝、8 线程共 512 次分配互不重叠。

## 四、03_free_list_allocator：把释放的内存变成链表

### 1. 侵入式节点与最小块大小

```rust
struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}
```

对象释放后，原对象占用的头部字节改作 `FreeBlock`。
因此，即使只申请 1 字节，分配器内部也必须至少保留一个节点大小，
且返回地址满足 `FreeBlock` 的对齐。

本实现统一计算：

```text
实际块大小 = 按节点对齐向上取整(max(申请大小, 节点大小))
实际对齐   = max(申请对齐, 节点对齐)
```

大小向节点对齐取整，也保证拆分出来的尾块可以写入新的节点。
所有取整加法都检查整数溢出。

### 2. first-fit 与“指向链接的指针”

分配时从表头查找第一个“大小足够且起点满足对齐”的空闲块。
使用 `link: *mut *mut FreeBlock` 统一处理表头和中间节点：

```text
link -> head                  或者 link -> previous.next
             \                               \
              current                         current
```

找到后更新 `*link`，就能摘除当前节点，不需要给头节点再写一套特殊逻辑。
遇到小块或对齐不符合的节点则继续遍历；没有合适节点时才使用 bump 区域。

### 3. 为什么增加尾块拆分

若释放一个 256 字节块，之后只申请 64 字节，直接交出整块会浪费剩余空间。
本实现额外加入简单拆分：

```text
原空闲块：[----------------- 256 B -----------------]
申请以后：[ 64 B 已分配 ][--------- 192 B 空闲 --------]
```

尾部至少容得下一个节点时，把它重新链接到原位置。
不足一个节点的尾部不再进入链表；后续释放只按本次布局记账，这一点小碎片会损失。
当前没有合并相邻空闲块，也没有回收 bump 对齐填充，因此仍可能有外部碎片。
它是教学型 first-fit 分配器，而不是完整通用堆实现。

### 4. 释放时的头插法

```text
block.next = 原 head
head = block
```

只需要写一个节点并更新表头，释放为 O(1)。查找分配为 O(f)，f 是空闲节点数。
释放必须传回本分配器当前仍存活的块及其原始 `Layout`；重复释放不在合法接口约定内。

### 5. 为什么重做锁的范围

模板测试分支只在读取、设置 head 的瞬间持有 `Mutex`，
遍历和摘链时锁已释放；生产分支的 `UnsafeCell` 本身更不提供线程同步。
所以不能仅因字段里出现了 Mutex 或 atomic 就认为整个算法线程安全。

本实现使用 `core` 自旋锁，持锁范围覆盖整个查找、摘链、拆分、头插和 bump 更新：

- 加锁使用 Acquire，解锁使用 Release。
- RAII `ListGuard` 在所有返回路径上释放锁。
- `UnsafeCell` 中的链表头和所有空闲节点仅在持锁时访问。
- bump 区域也由同一把锁保护，因此其原子读写用 Relaxed，不再需要 CAS。
- 生产路径和测试路径使用同一实现，避免“测试带锁、生产无锁”的差别。

这个锁不处理硬件中断重入：若用于真实内核，中断处理程序在同一 CPU 上
重入持锁的分配器会死锁，还需要中断屏蔽或其他内核级同步设计。
这里也未把该分配器注册为测试程序的全局分配器。

新增测试包括最小块回收、跳过小表头并摘除中间节点、对齐筛选、大块拆分、极大请求。
并发测试让 8 个线程先耗尽恰好 512 字节的 bump 区域，再反复回收分配 64 字节块，
验证内容未串写，最后全部 8 个块仍可再次取出，避免 bump 后备空间掩盖链表丢失。

## 五、04_syscall_wrapper：从寄存器约定到 Linux 调用

### 1. ABI 描述与真正执行是两层工作

三个 `*_abi()` 返回结构化的寄存器、指令和调用号知识；
`syscall3` 则依照 `cfg(target_arch, target_os)` 编译对应的汇编。
补齐了模板遗漏的 RISC-V 执行分支及其本机调用号。

| 架构 | 指令 | 编号寄存器 | 返回寄存器 | 前三个参数 | read/write/close/exit |
|---|---|---|---|---|---|
| x86_64 | syscall | rax | rax | rdi/rsi/rdx | 0/1/3/60 |
| aarch64 | svc #0 | x8 | x0 | x0/x1/x2 | 63/64/57/93 |
| riscv64 | ecall | a7 | a0 | a0/a1/a2 | 63/64/57/93 |

### 2. 汇编中要告诉编译器什么

以 x86_64 为例：

```rust
core::arch::asm!(
    "syscall",
    inlateout("rax") id => ret,
    in("rdi") arg0,
    in("rsi") arg1,
    in("rdx") arg2,
    lateout("rcx") _,
    lateout("r11") _,
    options(nostack),
);
```

- `inlateout` 表示同一个寄存器先传入编号，执行后取回返回值。
- `rcx`、`r11` 会被指令覆盖，显式声明，避免编译器错误保留其中的值。
- aarch64 / riscv64 的第一个参数寄存器同时也是返回值寄存器。
- 只使用 `nostack`，不使用 `nomem` / `readonly` / `pure`：
  `read` 会修改缓冲区，`write` 会读取缓冲区，调用还有外部副作用。

### 3. 四个包装函数

```text
sys_write(fd, buf) -> syscall3(WRITE, fd, buf.as_ptr(),     buf.len())
sys_read(fd, buf)  -> syscall3(READ,  fd, buf.as_mut_ptr(), buf.len())
sys_close(fd)     -> syscall3(CLOSE, fd, 0, 0)
sys_exit(code)    -> syscall3(EXIT,  code, 0, 0)
```

返回值是 Linux 原始 `isize`：成功时通常是字节数或零，错误时是负 errno，
例如非法描述符读操作在测试中返回 `-9`（`EBADF`）。
这里不转换为 `std::io::Result`，也不自动重试或保证一次读写完整缓冲区。

### 4. 实测遇到的 exit 陷阱

最初用子测试程序中的一个 libtest 测试调用 `sys_exit(37)`。
实际出现等待：原始 Linux `exit` 结束的是调用线程，而 libtest 的主线程
仍在等工作线程提交测试结果。这不是把返回类型写成 `!` 就能解决的问题。

最终改用 `fork` 创建只有调用线程的子进程，子进程中仅执行原始退出调用，
父进程用 `waitpid` 验证退出码 37，并处理等待被信号中断的情况。
原始 `exit` 不等同于 `exit_group`，也不负责运行 Rust 析构函数。

其他新增测试通过本地 UnixStream 对真实调用做往返读写，并显式转交 fd 所有权再 close，
避免测试对象析构时重复关闭相同描述符。

## 六、05_fd_table：稳定编号与共享所有权

采用：

```rust
Vec<Option<Arc<dyn File>>>
```

- 下标就是 fd；`Some` 是在用槽，`None` 是空槽。
- `alloc` 从左向右寻找第一个 `None`，否则 push 到末尾。
- `get` 用 `Vec::get` 检查边界，再 clone `Arc`。
- `close` 用 `get_mut` 和 `Option::take` 清空槽，返回是否真的移出了文件。
- `count` 统计所有 `Some`，不是返回向量长度。

为什么不使用 `Vec::remove`？删除会让后续元素左移，例如关闭 fd=1 后原 fd=2
会变成 fd=1，破坏所有已保存的描述符编号。

为什么返回 Arc 克隆？调用者拿到的文件句柄可以独立存活。关闭表中的 fd
只减少表持有的引用；最后一个 Arc 被释放才会销毁对象。
新增测试直接检查引用计数与关闭后的句柄可用性，也验证多个空槽时先复用最小编号。

`alloc/get/count` 中的线性扫描最坏为 O(n)，`close` 按下标取槽为 O(1)。
表的修改需要 `&mut self`；并发共享整个表时仍需外层同步。

## 七、完成过程与可复现验证

1. 检查根目录 workspace、题目注释和 `exercises.toml` 提示，确认目标是五个完整练习。
2. 保留原有测试，实现内存函数、CAS bump、空闲链表、三架构 ABI 与 fd 表。
3. 先运行原有 46 项测试，全部通过。
4. 增加 17 项回归测试，覆盖生产库编译路径、边界、并发和真实系统调用。
5. 排查退出测试等待问题，改为单线程子进程验证原始 exit。
6. 用 Clippy 检查代码，修正对齐判断的 lint；对刻意测试原始字节串的单个测试
   使用有说明的 `manual_c_str_literals` 局部允许项。
7. 验证 Debug、Release、严格 Clippy、格式及 RISC-V 库交叉构建。

在仓库根目录运行：

```bash
# 下列包选择参数可复用于 test/build/clippy
packages=(-p mem_primitives -p bump_allocator -p free_list_allocator -p syscall_wrapper -p fd_table)

cargo test "${packages[@]}"
cargo test "${packages[@]}" --release
cargo clippy "${packages[@]}" --all-targets -- -D warnings
cargo build "${packages[@]}" --target riscv64gc-unknown-linux-gnu

# 只检查本模块的格式，避免涉及其他练习
rustfmt --edition 2021 --check \
  exercises/02_no_std_dev/*/src/lib.rs \
  exercises/02_no_std_dev/*/tests/regressions.rs
```

测试统计（不把零项文档测试计入总数）：

| 包 | 原有单元测试 | 新增集成测试 | 合计 |
|---|---:|---:|---:|
| mem_primitives | 11 | 3 | 14 |
| bump_allocator | 6 | 3 | 9 |
| free_list_allocator | 5 | 6 | 11 |
| syscall_wrapper | 17 | 3 | 20 |
| fd_table | 7 | 2 | 9 |
| **总计** | **46** | **17** | **63** |

验证范围：宿主 x86_64 Linux 运行测试；RISC-V 生成库产物，验证条件编译分支和
汇编实际能通过编译。当前环境没有 QEMU / RISC-V 链接器，未运行 RISC-V 可执行测试。
aarch64 分支已实现，但未在该架构执行或交叉构建。没有把其他尚未完成模块的
workspace 全量测试计作本模块验收。

保留用户原有的 `01_concurrency_sync/04_process_pipe/src/diff.md` 修改，不纳入本次实现。

## 八、核对资料

已尝试按仓库要求使用 Context7，服务返回月度额度耗尽；随后核对官方资料：

- Rust `GlobalAlloc` 契约：https://doc.rust-lang.org/core/alloc/trait.GlobalAlloc.html
- Rust `alloc`：https://doc.rust-lang.org/alloc/index.html
- Rust 内联汇编：https://doc.rust-lang.org/reference/inline-assembly.html
- Linux x86_64 调用表：https://github.com/torvalds/linux/blob/master/arch/x86/entry/syscalls/syscall_64.tbl
- Linux 通用调用号：https://github.com/torvalds/linux/blob/master/include/uapi/asm-generic/unistd.h

阅读建议：先沿“字节操作 → 只分配 → 回收复用 → 系统调用 → 文件抽象”理解代码，
再阅读每题新增测试，从反例确认每一个安全约束为何必要。
