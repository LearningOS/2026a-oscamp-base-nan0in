//! # Tokio Async Tasks
//!
//! In this exercise, you will use `tokio::spawn` to create concurrent asynchronous tasks.
//!
//! ## Concepts
//! - `tokio::spawn` creates asynchronous tasks
//! - `JoinHandle` waits for task completion
//! - Concurrent execution between asynchronous tasks

use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

/// Concurrently compute the square of each number in 0..n, collect results and return in order.
///
/// Hint: Create `tokio::spawn` task for each i, collect JoinHandle, await them sequentially.
pub async fn concurrent_squares(n: usize) -> Vec<usize> {
    // 先把所有任务都 spawn 出去：spawn 之后任务就被丢进运行时的调度队列，
    // 主任务还没 await 的时候它们已经开始跑了 —— 这就是「并发」的来源。
    let mut handles: Vec<JoinHandle<usize>> = Vec::with_capacity(n);
    for i in 0..n {
        // async move 把 i 的所有权搬进任务闭包里。
        handles.push(tokio::spawn(async move { i * i }));
    }

    // 按创建顺序 await：JoinHandle 的 await 只是「等这个任务结束」，
    // 不会影响其它任务继续跑，所以仍然是并发。
    let mut results = Vec::with_capacity(n);
    for h in handles {
        // unwrap() 处理 JoinError（任务 panic 或运行时关闭）。
        results.push(h.await.unwrap());
    }
    results
}

/// Concurrently execute multiple "time-consuming" tasks (simulated with sleep), return all results.
/// Each task sleeps `duration_ms` milliseconds and then returns its `task_id`.
///
/// Key: All tasks should execute concurrently, total duration should be close to single task duration, not sum of all tasks.
pub async fn parallel_sleep_tasks(n: usize, duration_ms: u64) -> Vec<usize> {
    let mut handles: Vec<JoinHandle<usize>> = Vec::with_capacity(n);
    for id in 0..n {
        handles.push(tokio::spawn(async move {
            // sleep 是异步的：只把定时器注册到运行时，不阻塞线程，
            // 所以 n 个任务能真正「同时」睡。
            sleep(Duration::from_millis(duration_ms)).await;
            id
        }));
    }

    let mut results = Vec::with_capacity(n);
    for h in handles {
        results.push(h.await.unwrap());
    }
    // 完成顺序取决于调度，这里按 id 排序，保证返回值确定。
    results.sort();
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_squares_basic() {
        let result = concurrent_squares(5).await;
        assert_eq!(result, vec![0, 1, 4, 9, 16]);
    }

    #[tokio::test]
    async fn test_squares_zero() {
        let result = concurrent_squares(0).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_squares_one() {
        let result = concurrent_squares(1).await;
        assert_eq!(result, vec![0]);
    }

    #[tokio::test]
    async fn test_parallel_sleep() {
        let start = Instant::now();
        let result = parallel_sleep_tasks(5, 100).await;
        let elapsed = start.elapsed();

        assert_eq!(result, vec![0, 1, 2, 3, 4]);
        // Concurrent execution, total time should be much less than 5 * 100ms
        assert!(
            elapsed.as_millis() < 400,
            "Tasks should run concurrently, took {}ms",
            elapsed.as_millis()
        );
    }
}
