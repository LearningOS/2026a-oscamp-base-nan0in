diff --git a/exercises/01_concurrency_sync/04_process_pipe/src/lib.rs b/exercises/01_concurrency_sync/04_process_pipe/src/lib.rs
index 25f0db0..05446df 100644
--- a/exercises/01_concurrency_sync/04_process_pipe/src/lib.rs
+++ b/exercises/01_concurrency_sync/04_process_pipe/src/lib.rs
@@ -31,8 +31,8 @@
 //! Each function includes a `TODO` comment indicating where you need to write code.
 //! Run `cargo test` to check your implementations.
 
-use std::io::{self, Read, Write};
-use std::process::{Command, Stdio};
+use std::io::{self, BufReader, Read, Write,BufRead};
+use std::process::{Command, Output, Stdio};
 
 /// Execute the given shell command and return its stdout output.
 ///
@@ -53,7 +53,13 @@ pub fn run_command(program: &str, args: &[&str]) -> String {
     // TODO: Set stdout to Stdio::piped()
     // TODO: Execute with .output() and get output
     // TODO: Convert stdout to String and return
-    todo!()
+    let output = Command::new(program)
+        .args(args)
+        .stdout(Stdio::piped())
+        .output()
+        .unwrap();
+
+    String::from_utf8(output.stdout).unwrap()
 }
 
 /// Write data to child process (cat) stdin via pipe and read its stdout output.
@@ -89,7 +95,38 @@ pub fn pipe_through_cat(input: &str) -> String {
     // TODO: Write input to child process stdin
     // TODO: Drop stdin to close pipe (otherwise cat won't exit)
     // TODO: Read output from child process stdout
-    todo!()
+
+    // 子进程和双向管道创建
+    let mut child = Command::new("cat")
+        .stdin(Stdio::piped())
+        .stdout(Stdio::piped())
+        .spawn()
+        .unwrap();
+
+    let mut stdin = child
+        .stdin
+        .take()
+        .unwrap();
+    stdin
+        .write_all(input.as_bytes())
+        .unwrap();
+
+    drop(stdin);
+
+    let mut output = String::new();
+    let mut stdout = child
+        .stdout
+        .take()
+        .unwrap();
+    stdout
+        .read_to_string(&mut output)
+        .unwrap();
+
+    child
+        .wait()
+        .unwrap();
+
+    output
 }
 
 /// Get child process exit code.
@@ -110,7 +147,14 @@ pub fn get_exit_code(command: &str) -> i32 {
     // TODO: Use Command::new("sh").args(["-c", command])
     // TODO: Execute and get status
     // TODO: Return exit code
-    todo!()
+    let status = Command::new("sh")
+        .args(["-c", command])
+        .status()
+        .unwrap();
+
+    status
+        .code()
+        .unwrap_or(-1)
 }
 
 /// Execute the given shell command and return its stdout output as a `Result`.
@@ -131,13 +175,22 @@ pub fn get_exit_code(command: &str) -> i32 {
 /// 1. Create a `Command` with the given program and arguments.
 /// 2. Set `.stdout(Stdio::piped())`.
 /// 3. Call `.output()` and propagate any `io::Error`.
+//那我直接用?，成功提取出output否则传播回io::Error
 /// 4. Convert `stdout` to `String` with `String::from_utf8`; if that fails, map to an `io::Error`.
 pub fn run_command_with_result(program: &str, args: &[&str]) -> io::Result<String> {
     // TODO: Use Command::new to create process
     // TODO: Set stdout to Stdio::piped()
     // TODO: Execute with .output() and handle Result
     // TODO: Convert stdout to String with from_utf8, mapping errors to io::Error
-    todo!()
+    let output = Command::new(program)
+        .args(args)
+        .stdout(Stdio::piped())
+        .output()?;
+
+    let result = String::from_utf8(output.stdout)
+        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
+
+    Ok(result)
 }
 
 /// Interact with `grep` via bidirectional pipes, filtering lines that contain a pattern.
@@ -167,7 +220,43 @@ pub fn pipe_through_grep(pattern: &str, input: &str) -> String {
     // TODO: Drop stdin to close pipe
     // TODO: Read output from child stdout line by line
     // TODO: Collect and return matching lines
-    todo!()
+    let mut child = Command::new("grep")
+        .arg(pattern)
+        .stdin(Stdio::piped())
+        .stdout(Stdio::piped())
+        .spawn()
+        .unwrap();
+
+    //向子进程写入数据
+    let mut stdin = child.stdin.take().unwrap();
+
+    for line in input.split_inclusive('\n'){
+        stdin.write_all(line.as_bytes()).unwrap();
+    }
+
+    drop(stdin);
+
+    //逐行读取grep输出
+    let stdout = child.stdout.take().unwrap();
+    let reader = BufReader::new(stdout);
+
+    let mut result=String::new();
+    let mut line =String::new();
+
+    loop {
+        line.clear();
+        let bytes_read = reader.read_line(&mut line).unwrap();
+        if bytes_read == 0 {
+            break;
+        }
+        result.push_str(&line);
+    }
+
+
+
+    child.wait().unwrap();
+
+    result
 }
 
 #[cfg(test)]
@@ -212,7 +301,12 @@ mod tests {
     fn test_run_command_with_result_success() {
         let result = run_command_with_result("echo", &["hello"]);
         assert!(result.is_ok());
-        assert_eq!(result.unwrap().trim(), "hello");
+        assert_eq!(
+            result
+                .unwrap()
+                .trim(),
+            "hello"
+        );
     }
 
     #[test]
