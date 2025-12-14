use unidiff::PatchSet;

fn main() {
    let test_diff = r#"diff --git a/src/main.rs b/src/main.rs
index 0123456..789abcd 100644
---	a/src/main.rs
+++	b/src/main.rs
@@ -1,5 +1,5 @@
fn main() {
-    println!("Hello, world!");
+    println!("Hello, Gemini!");
    println!("Another line");
}
"#;

    println!("Trying to parse diff:");
    println!("{}", test_diff);
    
    let mut patch_set = PatchSet::new();
    match patch_set.parse(test_diff) {
        Ok(()) => {
            println!("Parse successful! Found {} files", patch_set.files().count());
            for (i, file) in patch_set.files().enumerate() {
                println!("File {}: {}", i, file.target_file);
                for (j, hunk) in file.hunks().enumerate() {
                    println!("  Hunk {}: source_start={}, target_start={}", j, hunk.source_start, hunk.target_start);
                }
            }
        }
        Err(e) => {
            println!("Parse failed: {:?}", e);
        }
    }
}