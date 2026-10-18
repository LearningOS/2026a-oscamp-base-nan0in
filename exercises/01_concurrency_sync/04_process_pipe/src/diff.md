diff --git a/exercises/01_concurrency_sync/04_process_pipe/src/lib.rs b/exercises/01_concurrency_sync/04_process_pipe/src/lib.rs
index ab331d4..05446df 100644
--- a/exercises/01_concurrency_sync/04_process_pipe/src/lib.rs
+++ b/exercises/01_concurrency_sync/04_process_pipe/src/lib.rs
@@ -241,13 +241,19 @@ pub fn pipe_through_grep(pattern: &str, input: &str) -> String {
     let reader = BufReader::new(stdout);
 
     let mut result=String::new();
-
-    for line in reader.lines() {
-        let line=line.unwrap();
+    let mut line =String::new();
+
+    loop {
+        line.clear();
+        let bytes_read = reader.read_line(&mut line).unwrap();
+        if bytes_read == 0 {
+            break;
+        }
         result.push_str(&line);
-        result.push('\n');
     }
 
+
+
     child.wait().unwrap();
 
     result
