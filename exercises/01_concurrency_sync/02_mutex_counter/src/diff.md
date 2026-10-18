diff --git a/exercises/01_concurrency_sync/02_mutex_counter/src/lib.rs b/exercises/01_concurrency_sync/02_mutex_counter/src/lib.rs
index 171f7fa..4c08f4b 100644
--- a/exercises/01_concurrency_sync/02_mutex_counter/src/lib.rs
+++ b/exercises/01_concurrency_sync/02_mutex_counter/src/lib.rs
@@ -16,11 +16,34 @@ use std::thread;
 ///
 /// Hint: Use `Arc<Mutex<usize>>` as the shared counter.
 pub fn concurrent_counter(n_threads: usize, count_per_thread: usize) -> usize {
-    // TODO: Create Arc<Mutex<usize>> with initial value 0
-    // TODO: Spawn n_threads threads
-    // TODO: In each thread, lock() and increment count_per_thread times
-    // TODO: Join all threads, return final value
-    todo!()
+    // 创建共享计数器
+    let counter = Arc::new(Mutex::new(0usize));
+    let mut handles = Vec::new();
+
+    // 创建n个线程
+    for _ in 0..n_threads {
+        let counter = Arc::clone(&counter);
+
+        let handle = thread::spawn(move || {
+            for _ in 0..count_per_thread {
+                let mut num = counter
+                    .lock()
+                    .unwrap();
+                *num += 1;
+            }
+        });
+        handles.push(handle);
+    }
+    for handle in handles {
+        handle
+            .join()
+            .unwrap();
+    }
+    let result = *counter
+        .lock()
+        .unwrap();
+
+    result
 }
 
 /// Add elements to a shared vector concurrently using multiple threads.
@@ -32,7 +55,26 @@ pub fn concurrent_collect(n_threads: usize) -> Vec<usize> {
     // TODO: Create Arc<Mutex<Vec<usize>>>
     // TODO: Each thread pushes its own id
     // TODO: After joining all threads, sort the result and return
-    todo!()
+    let counter = Arc::new(Mutex::new(Vec::new()));
+    let mut handles = Vec::new();
+
+    for id in 0..n_threads{
+        let counter = Arc::clone(&counter);
+        let handle = thread::spawn(move || {
+            let mut num =counter.lock().unwrap();
+            num.push(id);
+
+        });
+        handles.push(handle);
+    }
+    for handle in handles{
+        handle.join().unwrap();
+    }
+    let mut result = counter.lock().unwrap().clone();
+    result.sort();
+
+    result
+    
 }
 
 #[cfg(test)]
