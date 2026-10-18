diff --git a/exercises/01_concurrency_sync/01_thread_spawn/src/lib.rs b/exercises/01_concurrency_sync/01_thread_spawn/src/lib.rs
index cbb0d16..b728b4c 100644
--- a/exercises/01_concurrency_sync/01_thread_spawn/src/lib.rs
+++ b/exercises/01_concurrency_sync/01_thread_spawn/src/lib.rs
@@ -154,10 +154,16 @@ use std::time::Duration;
 /// Hint: Use `thread::spawn` and `move` closure.
 #[allow(unused_variables)]
 pub fn double_in_thread(numbers: Vec<i32>) -> Vec<i32> {
-    // TODO: Create a new thread to multiply each element of numbers by 2
-    // Use thread::spawn and move closure
-    // Use join().unwrap() to get result
-    todo!()
+    let handle = thread::spawn(move || {
+        numbers
+            .into_iter()
+            .map(|x| x * 2)
+            .collect()
+    });
+
+    handle
+        .join()
+        .unwrap()
 }
 
 /// Sum two vectors in parallel, returning a tuple of two sums.
@@ -165,9 +171,25 @@ pub fn double_in_thread(numbers: Vec<i32>) -> Vec<i32> {
 /// Hint: Create two threads for each vector.
 #[allow(unused_variables)]
 pub fn parallel_sum(a: Vec<i32>, b: Vec<i32>) -> (i32, i32) {
-    // TODO: Create two threads to sum a and b respectively
-    // Join both threads to get results
-    todo!()
+    let (handle_a, handle_b) = {
+        thread::scope(|s| {
+            let h1 = thread::spawn(move || {
+                a.iter()
+                    .sum::<i32>()
+            });
+            let h2 = thread::spawn(move || {
+                b.iter()
+                    .sum::<i32>()
+            });
+            (
+                h1.join()
+                    .unwrap(),
+                h2.join()
+                    .unwrap(),
+            )
+        })
+    };
+    (handle_a, handle_b)
 }
 
 // ============================================================================
@@ -185,7 +207,17 @@ pub fn named_sleeper(value: i32, ms: u64) -> i32 {
     // TODO: Create a thread builder with name "sleeper"
     // TODO: Spawn a thread that sleeps for `ms` milliseconds and returns `value`
     // TODO: Join the thread and return the value
-    todo!()
+    let builder = thread::Builder::new().name("sleeper".into());
+    let handle = builder
+        .spawn(move || {
+            thread::sleep(Duration::from_millis(ms));
+            value
+        })
+        .unwrap();
+
+    handle
+        .join()
+        .unwrap()
 }
 
 thread_local! {
@@ -200,7 +232,11 @@ thread_local! {
 /// Hint: Use `THREAD_COUNT.with(|cell| { ... })` to access the thread‑local variable.
 pub fn increment_thread_local() -> usize {
     // TODO: Use THREAD_COUNT.with to increment and return the new count
-    todo!()
+    THREAD_COUNT.with(|cell| {
+        let mut count = cell.borrow_mut();
+        *count += 1;
+        *count
+    })
 }
 
 /// Spawn two threads using a **scoped thread** to compute the sum of two slices without moving ownership.
@@ -213,10 +249,23 @@ pub fn increment_thread_local() -> usize {
 /// making the borrow safe.
 #[allow(unused_variables)]
 pub fn scoped_slice_sum(a: &[i32], b: &[i32]) -> (i32, i32) {
-    // TODO: Use thread::scope to spawn two threads
-    // TODO: Each thread sums its slice
-    // TODO: Wait for both threads and return the results
-    todo!()
+    let (sum_a, sum_b) = thread::scope(|s| {
+        let h1 = s.spawn(|| {
+            a.iter()
+                .sum::<i32>()
+        });
+        let h2 = s.spawn(|| {
+            b.iter()
+                .sum::<i32>()
+        });
+        (
+            h1.join()
+                .unwrap(),
+            h2.join()
+                .unwrap(),
+        )
+    });
+    (sum_a, sum_b)
 }
 
 /// Handle a possible panic in a spawned thread.
@@ -231,9 +280,18 @@ pub fn scoped_slice_sum(a: &[i32], b: &[i32]) -> (i32, i32) {
 /// In this exercise, the inner type is just `i32`, not a `Result`.
 #[allow(unused_variables)]
 pub fn handle_panic(value: i32, should_panic: bool) -> Result<i32, ()> {
-    // TODO: Spawn a thread that either panics or returns value
-    // TODO: Join and map the result appropriately
-    todo!()
+    let handle = thread::spawn(move || {
+        if should_panic {
+            panic!("oops");
+        } else {
+            value
+        }
+    });
+
+    match handle.join() {
+        Ok(v) => Ok(v),
+        Err(_) => Err(()),
+    }
 }
 
 #[cfg(test)]
@@ -290,13 +348,19 @@ mod tests {
             handles.push(thread::spawn(move || {
                 let v1 = increment_thread_local();
                 let v2 = increment_thread_local();
-                counters.lock().unwrap().push((v1, v2));
+                counters
+                    .lock()
+                    .unwrap()
+                    .push((v1, v2));
             }));
         }
         for h in handles {
-            h.join().unwrap();
+            h.join()
+                .unwrap();
         }
-        let results = counters.lock().unwrap();
+        let results = counters
+            .lock()
+            .unwrap();
         // Each thread should have counted (1, 2) independently.
         assert_eq!(results.len(), 2);
         assert!(results.contains(&(1, 2)));
