//! # Manual Future Implementation
//!
//! In this exercise, you will manually implement the `Future` trait for custom types to understand the core mechanism of asynchronous runtime.
//!
//! ## Concepts
//! - `std::future::Future` trait
//! - `Poll::Ready` and `Poll::Pending`
//! - The role of `Waker`: notifying the runtime to poll again

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Countdown Future: decrements count by 1 each time it's polled,
/// returns `"liftoff!"` when count reaches 0.
pub struct CountDown {
    pub count: u32,
}

impl CountDown {
    pub fn new(count: u32) -> Self {
        Self { count }
    }
}

// TODO: Implement Future trait for CountDown
// - Output type is &'static str
// - Each poll: if count == 0, return Poll::Ready("liftoff!")
// - Otherwise count -= 1, call cx.waker().wake_by_ref(), return Poll::Pending
//
// Hint: Use `self.get_mut()` to get `&mut Self` (since self is Pin<&mut Self>)
impl Future for CountDown {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 先拿到 &mut Self。Pin 保证对象不会被移动，
        // 而 CountDown 本身没有任何自引用，所以 get_mut() 是安全的。
        let this = self.get_mut();

        if this.count == 0 {
            // 倒数结束，交出结果。
            Poll::Ready("liftoff!")
        } else {
            this.count -= 1;
            // 关键：告诉执行器「我有进展了，请再 poll 我一次」。
            // 否则执行器会一直挂起，await 永远不返回。
            cx.waker()
                .wake_by_ref();
            Poll::Pending
        }
    }
}

/// Yield-only-once Future: first poll returns Pending, second returns Ready(()).
/// This is the minimal example of an asynchronous state machine.
pub struct YieldOnce {
    yielded: bool,
}

impl YieldOnce {
    pub fn new() -> Self {
        Self { yielded: false }
    }
}

// TODO: Implement Future trait for YieldOnce
// - Output type is ()
// - First poll: set yielded = true, wake waker, return Pending
// - Second poll: return Ready(())
impl Future for YieldOnce {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if this.yielded {
            // 第二次 poll：让出一次就够了，直接完成。
            Poll::Ready(())
        } else {
            this.yielded = true;
            // 第一次 poll：先挂起，再唤醒自己，保证很快会被再次 poll。
            cx.waker()
                .wake_by_ref();
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_countdown_zero() {
        let result = CountDown::new(0).await;
        assert_eq!(result, "liftoff!");
    }

    #[tokio::test]
    async fn test_countdown_three() {
        let result = CountDown::new(3).await;
        assert_eq!(result, "liftoff!");
    }

    #[tokio::test]
    async fn test_yield_once() {
        YieldOnce::new().await;
    }

    #[tokio::test]
    async fn test_countdown_large() {
        let result = CountDown::new(100).await;
        assert_eq!(result, "liftoff!");
    }
}
