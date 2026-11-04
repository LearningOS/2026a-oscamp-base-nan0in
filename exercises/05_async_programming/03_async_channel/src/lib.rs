//! # Async Channel
//!
//! In this exercise, you will use `tokio::sync::mpsc` async channels to implement producer-consumer pattern.
//!
//! ## Concepts
//! - `tokio::sync::mpsc::channel` creates bounded async channels
//! - Async `send` and `recv`
//! - Channel closing mechanism (receiver returns None after all senders are dropped)

use tokio::sync::mpsc;

/// Async producer-consumer:
/// - Create a producer task that sends each element from items sequentially
/// - Create a consumer task that receives all elements and collects them into Vec for return
///
/// Hint: Set channel capacity to items.len().max(1)
pub async fn producer_consumer(items: Vec<String>) -> Vec<String> {
    // 有界通道：容量至少为 1（tokio 不允许 0 容量）。
    let (tx, mut rx) = mpsc::channel::<String>(items.len().max(1));

    // 生产者任务：把 items 搬进去逐个 send。send 是 async 的：
    // 通道满时会挂起任务而不是阻塞线程。
    tokio::spawn(async move {
        for item in items {
            tx.send(item)
                .await
                .unwrap();
        }
        // tx 在这里被 drop，接收端随后会收到 None，循环自然结束。
    });

    // 消费者：在当前任务里直接收，直到通道关闭。
    let mut result = Vec::new();
    while let Some(item) = rx.recv().await {
        result.push(item);
    }
    result
}

/// Fan‑in pattern: multiple producers, one consumer.
/// Create `n_producers` producers, each sending `"producer {id}: message"`.
/// Consumer collects all messages, sorts them, and returns.
pub async fn fan_in(n_producers: usize) -> Vec<String> {
    let (tx, mut rx) = mpsc::channel::<String>(n_producers.max(1));

    for id in 0..n_producers {
        // 每个生产者拿一份自己的 Sender 克隆。
        let tx = tx.clone();
        tokio::spawn(async move {
            let msg = format!("producer {id}: message");
            tx.send(msg)
                .await
                .unwrap();
        });
    }

    // 关键一步：把原始 tx 扔掉。
    // 否则 rx.recv().await 会因为它一直「还没关闭」而永远等下去。
    drop(tx);

    let mut result = Vec::new();
    while let Some(msg) = rx.recv().await {
        result.push(msg);
    }
    // 多个生产者完成的顺序不确定，排序让输出可复现。
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_producer_consumer() {
        let items = vec!["hello".into(), "async".into(), "world".into()];
        let result = producer_consumer(items.clone()).await;
        assert_eq!(result, items);
    }

    #[tokio::test]
    async fn test_producer_consumer_empty() {
        let result = producer_consumer(vec![]).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_fan_in() {
        let result = fan_in(3).await;
        assert_eq!(
            result,
            vec![
                "producer 0: message",
                "producer 1: message",
                "producer 2: message",
            ]
        );
    }

    #[tokio::test]
    async fn test_fan_in_single() {
        let result = fan_in(1).await;
        assert_eq!(result, vec!["producer 0: message"]);
    }
}
