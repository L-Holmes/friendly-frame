use website_compiler::components::*;
use std::collections::HashSet;
use std::fs::{self, File, write, create_dir_all, read_to_string, remove_dir_all, OpenOptions};
use std::io::{Write, Result};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, SystemTime};
use tempfile::{tempdir, TempDir};
use std::env;

// A helper function to create a unique temporary directory for each test run.
fn setup_test_dir() -> PathBuf {
    let temp_dir = std::env::temp_dir().join(format!(
        "test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).unwrap();
    temp_dir
}

/////////////////////

#[test]
fn test_read_last_compile_time_existing_file() {
    // (in a temp dir, since tests run in parallel and would otherwise share the same file)
    let tmp = tempdir().unwrap();
    let test_file_path = tmp.path().join("test_compile_time.txt");
    let test_file: &str = test_file_path.to_str().unwrap();

    // Ensure the file is created and write the expected value
    write(test_file, "1627890000").unwrap();

    // Read the value from the file
    let result: u64 = read_last_compile_time(test_file);

    // Check if the read value matches the expected value
    assert_eq!(result, 1627890000);
}

#[test]
fn test_read_last_compile_time_non_existent_file() {
    let result: u64 = read_last_compile_time("non_existent_file.txt");
    assert_eq!(result, 0);
}

#[test]
fn test_update_last_compile_time() -> Result<()> {
    let tmp = tempdir()?;
    let test_file_path = tmp.path().join("test_compile_time.txt");
    let test_file: &str = test_file_path.to_str().unwrap();
    update_last_compile_time(1627890000, test_file)?;

    let content: String = read_to_string(test_file)?;
    assert_eq!(content.trim(), "1627890000");
    Ok(())
}

#[test]
fn test_files_in_source_dir() {
    // (in a temp dir, so it never touches the real edit-me/)
    let tmp = tempdir().unwrap();
    let source_dir = tmp.path().join("edit-me");
    create_dir_all(source_dir.join("test-dir")).unwrap();
    write(source_dir.join("test-dir/file1.txt"), "test content").unwrap();

    let files: Vec<PathBuf> = files_in_source_dir(source_dir.to_str().unwrap()).unwrap();
    assert!(files.iter().any(|f| f.ends_with("test-dir/file1.txt")));
}

#[test]
fn test_only_modify_time_newer_than_last_compile_time() {
    eprintln!("1");
    // 1. Setup
    let test_dir = setup_test_dir();
    let mut all_files = Vec::new();

    // Create some "old" files
    let old_file_1 = test_dir.join("old_file_1.txt");
    File::create(&old_file_1).unwrap();
    all_files.push(old_file_1);

    let old_file_2_to_modify = test_dir.join("old_file_2.txt");
    File::create(&old_file_2_to_modify).unwrap();
    all_files.push(old_file_2_to_modify.clone());

    // Wait a moment to ensure modification times are distinct
    // (50ms, since file timestamps can lag the system clock by a few ms)
    thread::sleep(Duration::from_millis(50));

    eprintln!("2");
    // 2. Define the "last compile time" before new changes (in millis, like .last_compiled)
    let last_compile_time: u64 = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;

    // Wait again so new files are definitely newer
    thread::sleep(Duration::from_millis(50));

    eprintln!("3");
    // 3. Create "new" files and modify an old one
    let new_file_1 = test_dir.join("new_file_1.txt");
    File::create(&new_file_1).unwrap();
    all_files.push(new_file_1.clone());

    let new_file_2 = test_dir.join("new_file_2.txt");
    File::create(&new_file_2).unwrap();
    all_files.push(new_file_2.clone());

    // Modify one of the old files to make it "new"
    let mut file_to_update = OpenOptions::new().write(true).open(&old_file_2_to_modify).unwrap();
    file_to_update.write_all(b"some data").unwrap();
    file_to_update.sync_all().unwrap(); // Ensure the modification is written to disk

    eprintln!("4");
    // 4. Run the function
    let result = only_modify_time_newer_than_last_compile_time(&all_files, last_compile_time).unwrap();

    eprintln!("5");
    // 5. Assert the results are correct
    assert_eq!(result.len(), 3, "Should have found 3 newer files");
    assert!(result.contains(&new_file_1));
    assert!(result.contains(&new_file_2));
    assert!(result.contains(&old_file_2_to_modify)); // This one was modified after the timestamp
    eprintln!("6");

    // Cleanup
    remove_dir_all(test_dir).unwrap();
    eprintln!("7");
}

// === NEW TESTS FOR get_relative_path ===

#[test]
fn test_get_relative_path_deeply_nested() {
    // Path has 5 segments: 'level1', 'level2', 'level3', 'level4', 'file.txt'
    // Expected output count: 5 - 2 = 3
    let path = Path::new("level1/level2/level3/level4/file.txt");
    assert_eq!(get_relative_path(path), "../../../");
}

#[test]
fn test_get_relative_path_medium_nesting() {
    // Path has 3 segments: 'docs', 'guides', 'index.md'
    // Expected output count: 3 - 2 = 1
    let path = Path::new("docs/guides/index.md");
    assert_eq!(get_relative_path(path), "../");
}

#[test]
fn test_get_relative_path_shallow_path() {
    // Path has 2 segments: 'src', 'main.rs'
    // Expected output count: 2 - 2 = 0
    let path = Path::new("src/main.rs");
    assert_eq!(get_relative_path(path), "");
}

#[test]
fn test_get_relative_path_root_file() {
    // Path has 1 segment: 'README.md'
    // Expected output count: 1 - 2 = 0 (saturating)
    let path = Path::new("README.md");
    assert_eq!(get_relative_path(path), "");
}

#[test]
fn test_get_relative_path_with_trailing_slash() {
    // Path has 4 segments: 'a', 'b', 'c', 'd'
    // Expected output count: 4 - 2 = 2
    // The trailing slash creates an empty component, which is correctly ignored by the filter.
    let path = Path::new("a/b/c/d/");
    assert_eq!(get_relative_path(path), "../../");
}

// === REPLACING COMPONENT PLACEHOLDERS ===

#[test]
fn test_replace_root_placeholder_with_relative_path_new() {
    let test_dir = setup_test_dir();
    let deep_dir = test_dir.join("a").join("b").join("c");
    fs::create_dir_all(&deep_dir).unwrap();
    let target_file = deep_dir.join("style.css");

    let file_content = "body { background: url('<root>/images/bg.png'); }";
    fs::write(&target_file, file_content).unwrap();

    replace_root_placeholder_with_relative_path_new("<root>", "<root>", &target_file).unwrap();

    let updated_content = fs::read_to_string(&target_file).unwrap();
    let relative_path_for_test = get_relative_path(&target_file);
    let expected_content = format!("body {{ background: url('{}images/bg.png'); }}", relative_path_for_test);

    assert_eq!(updated_content, expected_content);

    fs::remove_dir_all(test_dir).unwrap();
}

// === JAVASCRIPT COMPONENTS ===

#[test]
fn test_add_reusable_javascript_components() {
    let test_dir = setup_test_dir();
    let js_file = test_dir.join("app.js");
    let shared_dir = test_dir.join("shared").join("js_libs");
    fs::create_dir_all(&shared_dir).unwrap();

    let component1_path = shared_dir.join("component1.ts");
    fs::write(component1_path, "const x = 1;").unwrap();
    let component2_path = shared_dir.join("component2.ts");
    fs::write(component2_path, "function doIt() { console.log('hello'); }").unwrap();

    let main_js_content = "console.log('start');\n<js-component1>\nconsole.log('middle');\n<js-component2>\nconsole.log('end');";
    fs::write(&js_file, main_js_content).unwrap();

    add_reusable_javascript_components(&js_file, &test_dir, "js_libs", "<js-", ">").unwrap();

    let expected_content = "console.log('start');\nconst x = 1;\nconsole.log('middle');\nfunction doIt() { console.log('hello'); }\nconsole.log('end');";
    let actual_content = fs::read_to_string(js_file).unwrap();
    assert_eq!(actual_content, expected_content);

    fs::remove_dir_all(test_dir).unwrap();
}

// === HTML COMPONENTS ===

#[test]
fn test_replace_html_components_all_scenarios() {
    let output_dir = setup_test_dir();
    let src_dir = setup_test_dir();
    let html_file = output_dir.join("index.html");

    // --- Setup the initial HTML file with the original, complex parameter syntax ---
    let main_html = r#"<html><head><title>Test</title></head><body><r-card title='Welcome!' icon=@template:icon image='star.svg'></body></html>"#;
    fs::write(&html_file, main_html).unwrap();

    // --- Setup all source component files ---

    let card_src_dir = src_dir.join("shared/reusables/card");
    fs::create_dir_all(&card_src_dir).unwrap();
    fs::write(card_src_dir.join("card.html"), r#"<div><h2>{title}</h2>{icon}</div>"#).unwrap();
    fs::write(card_src_dir.join("card.scss"), "/* card.css */").unwrap();


    let icon_src_dir = src_dir.join("shared/reusables/icon");
    fs::create_dir_all(&icon_src_dir).unwrap();
    fs::write(icon_src_dir.join("icon.html"), r#"
        <img src='{image}'>
        "#).unwrap();
    fs::write(icon_src_dir.join("icon.ts"), "/* icon.js */").unwrap();


    // zeroth iteration:
    // -----------------------
    // <r-card title='Welcome!' icon="@template:icon image='star.svg'">
    // -----------------------
    // first iteration:
    // -----------------------
    // <div>
    //      <h2>{title}</h2>
    //      {icon}
    // </div>
    // vvvvvvvvvvvvvv [then it hit the params code, knowing: title='Welcome', icon=<icon template> and image='star.svg']
    // <div>
    //      <h2>Welcome!</h2>
    //      <r-icon>
    // </div>
    // -----------------------
    // Second iteration: It then checks to see if there are any more <r- .. > entries... There are, so it subs in;
    // !!! But it doesn't know what the image is! !!! TODO HOW TO MAKE IT KNOW WHAT THE IMAGE IS???!?!??!?
    // --> need some way of carrying into the next iteration...
    // -----------------------
    // <div>
    //      <h2>Welcome!</h2>
    //      <img src='{image}'>
    // </div>
    // -----------------------
    // It then checks to see if there are any more <r- .. > entries... There are not, so it doesn't continue.

    // --- Run the function ONCE. Its internal `loop` must handle all recursion. ---
    eprintln!("a");
    replace_html_component_placeholders(
        &html_file,
        &src_dir,
        "<r-",
        ">",
        "<root>",
        "@template:",
        "@none:",
        "{",
        "}",
        "[",
        "]",
    )
    .unwrap();
    eprintln!("b");

    // --- Assert the final state ---
    let final_content = fs::read_to_string(&html_file).unwrap();

    // 1. Check for all required CSS/JS links from all processed components (card and icon).
    assert!(final_content.contains(r#"<link rel="stylesheet" href="<root>/shared/reusables/card/card.css">"#));
    assert!(final_content.contains(r#"<script defer src="<root>/shared/reusables/icon/icon.js">"#));

    // 2. Check for the final, fully resolved HTML.
    let expected_welcome = r#"<h2>Welcome!</h2>"#;
    let expected_img = r#"<img src='star.svg'>"#;
    assert!( final_content.contains(expected_welcome), "Assertion failed: expected content not found. final_content: '{}', expected_body: '{}'", final_content, expected_welcome);
    assert!( final_content.contains(expected_img), "Assertion failed: expected content not found. final_content: '{}', expected_body: '{}'", final_content, expected_img);
    
    // 3. Ensure no component tags are left.
    assert!(!final_content.contains("<r-"));

    fs::remove_dir_all(output_dir).unwrap();
    fs::remove_dir_all(src_dir).unwrap();
}


// ===================================================================
// # === Tests for `get_all_files_that_need_recompiling` ===
// ===================================================================

// --- Helper Structs and Functions for Testing `get_all_files_that_need_recompiling` ---

// A structure to define a file to be created for a test.
struct TestFile<'a> {
    path: &'a str,
    content: &'a str,
}

// Test Harness Function: This is a powerful helper for creating a temporary,
// realistic file/directory structure for our tests to run against.
// It prevents tests from interfering with each other and from leaving messy files
// on the developer's machine.
//
// Returns a tuple:
//   - `TempDir`: A handle to the temporary directory. It's crucial to keep this in scope,
//     as the directory is automatically deleted when this handle is dropped.
//   - `PathBuf`: The root path of the created `source_dir` (e.g., `/tmp/xyz/edit-me`).
fn setup_test_environment(files: &[TestFile]) -> (TempDir, PathBuf) {
    // 1. Create a unique temporary directory for this specific test run.
    let tmp_dir = tempdir().expect("Failed to create temporary directory for test");
    
    // 2. Define our base `source_dir` inside the temporary directory.
    let source_dir = tmp_dir.path().join("edit-me");
    fs::create_dir_all(&source_dir).expect("Failed to create source_dir");

    // 3. Iterate through the provided file definitions and create them on the disk.
    for file in files {
        let full_path = source_dir.join(file.path);
        
        // Create parent directories if they don't exist (e.g., `pages/` or `shared/reusables/`).
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories for test file");
        }
        
        // Write the specified content to the file.
        fs::write(&full_path, file.content)
            .expect(&format!("Failed to write test file: {:?}", full_path));
    }
    
    // 4. Return the directory handle and the path to our `source_dir`.
    (tmp_dir, source_dir)
}

// Test Case 5: Basic dependency check.
// Scenario: `index.html` uses the `top-bar` component. If `top-bar.html` is modified,
// the function should detect that `index.html` also needs to be recompiled.
#[test]
fn test_recompile_basic_dependency() {
    println!("--- Running Test: test_recompile_basic_dependency ---");

    // --- ARRANGE ---
    // 1. Define the file system state for this test.
    let files_to_create = vec![
        TestFile { path: "shared/reusables/top-bar/top-bar.html", content: "<!-- Top Bar HTML -->" },
        TestFile { path: "pages/index.html", content: "<html><body><r-top-bar></body></html>" },
        TestFile { path: "pages/about.html", content: "<html><body><h1>About Us</h1></body></html>" },
    ];
    let (_tmp_dir, source_dir) = setup_test_environment(&files_to_create);
    
    // 2. Define the input for our function.
    // The initially modified file is just the top-bar component.
    let initial_modified_files = source_dir
        .join("shared/reusables/top-bar/top-bar.html")
        .to_string_lossy()
        .to_string();
    
    let components_dir = source_dir.join("shared/reusables");

    // --- ACT ---
    // 3. Execute the function.
    let result = get_all_files_that_need_recompiling(
        &source_dir.to_string_lossy(),
        "<r-",
        ">",
        &HashSet::from([initial_modified_files.clone()]),
        &components_dir.to_string_lossy(),
    );

    // --- ASSERT ---
    // 4. Verify the output.
    // We expect the final list to contain BOTH the original file AND the file that depends on it.
    let expected_files: HashSet<String> = HashSet::from([
        initial_modified_files.clone(),
        source_dir.join("pages/index.html").to_string_lossy().to_string(),
    ]);
    
    println!("Initial files:\n{}", &initial_modified_files);
    println!("Expected final list:\n{:?}", expected_files);
    println!("Actual final list:\n{:?}", result);

    assert_eq!(result, expected_files);
    println!("--- Test Passed ---\n");
}


// Test Case 6: Cascading dependency check (A -> B -> C).
// Scenario: `contact.html` uses `page-layout`. `page-layout` uses `top-bar`.
// If `top-bar.html` is modified, the function must recursively find all dependents.
// It should identify that `page-layout` needs to be updated, and because `page-layout` changed,
// `contact.html` must ALSO be updated.
#[test]
fn test_recompile_cascading_dependency() {
    println!("--- Running Test: test_recompile_cascading_dependency ---");
    
    // --- ARRANGE ---
    let files_to_create = vec![
        // Level C (the root change)
        TestFile { path: "shared/reusables/top-bar/top-bar.html", content: "<!-- Top Bar HTML -->" },
        // Level B (depends on C)
        TestFile { path: "shared/reusables/page-layout/page-layout.html", content: "<div><r-top-bar></div>" },
        // Level A (depends on B)
        TestFile { path: "pages/contact.html", content: "<html><r-page-layout></html>" },
        // Unrelated file
        TestFile { path: "pages/index.html", content: "<html>...</html>" },
    ];
    let (_tmp_dir, source_dir) = setup_test_environment(&files_to_create);
    
    // The only initially modified file is top-bar.html
    let initial_modified_files = source_dir
        .join("shared/reusables/top-bar/top-bar.html")
        .to_string_lossy()
        .to_string();
        
    let components_dir = source_dir.join("shared/reusables");

    // --- ACT ---
    let result = get_all_files_that_need_recompiling(
        &source_dir.to_string_lossy(),
        "<r-",
        ">",
        &HashSet::from([initial_modified_files.clone()]),
        &components_dir.to_string_lossy(),
    );

    // --- ASSERT ---
    // The final list must include all three files in the dependency chain.
    let expected_files: HashSet<String> = HashSet::from([
        source_dir.join("shared/reusables/top-bar/top-bar.html").to_string_lossy().to_string(),
        source_dir.join("shared/reusables/page-layout/page-layout.html").to_string_lossy().to_string(),
        source_dir.join("pages/contact.html").to_string_lossy().to_string(),
    ]);
    
    println!("Initial files:\n{}", &initial_modified_files);
    println!("Expected final list:\n{:?}", expected_files);
    println!("Actual final list:\n{:?}", result);
    
    assert_eq!(result, expected_files);
    println!("--- Test Passed ---\n");
}


// Test Case 7: Edge Case - Tag with extra whitespace and arguments.
// The regex should be robust enough to find the component name even if the tag
// is formatted unusually.
#[test]
fn test_recompile_tag_with_extra_whitespace_and_args() {
    println!("--- Running Test: test_recompile_tag_with_extra_whitespace_and_args ---");

    // --- ARRANGE ---
    let files_to_create = vec![
        TestFile { path: "shared/reusables/button/button.html", content: "<!-- button -->" },
        // Note the messy but valid tag format below.
        TestFile { path: "pages/index.html", content: r#"<body><r-  button arg1="value"  ></body>"# },
    ];
    let (_tmp_dir, source_dir) = setup_test_environment(&files_to_create);
    
    let initial_modified_files = source_dir.join("shared/reusables/button/button.html").to_string_lossy().to_string();
    let components_dir = source_dir.join("shared/reusables");

    // --- ACT ---
    let result = get_all_files_that_need_recompiling(
        &source_dir.to_string_lossy(),
        "<r-",
        ">",
        &HashSet::from([initial_modified_files.clone()]),
        &components_dir.to_string_lossy(),
    );

    // --- ASSERT ---
    // Should correctly identify `index.html` as a dependent file.
    let expected_files: HashSet<String> = HashSet::from([
        initial_modified_files.clone(),
        source_dir.join("pages/index.html").to_string_lossy().to_string(),
    ]);
    
    println!("Expected final list:\n{:?}", expected_files);
    println!("Actual final list:\n{:?}", result);
    
    assert_eq!(result, expected_files);
    println!("--- Test Passed ---\n");
}

// Test Case 8: Circular dependency check (A -> B, B -> A).
// A well-behaved function should handle this gracefully and terminate, not enter an infinite loop.
// Our implementation using a HashSet naturally handles this: once a file is in the set,
// it can't be added again, so the loop terminates when no *new* files can be found.
#[test]
fn test_recompile_circular_dependency() {
    println!("--- Running Test: test_recompile_circular_dependency ---");
    
    // --- ARRANGE ---
    let files_to_create = vec![
        // component-a depends on component-b
        TestFile { path: "shared/reusables/component-a/component-a.html", content: "<r-component-b>" },
        // component-b depends on component-a
        TestFile { path: "shared/reusables/component-b/component-b.html", content: "<r-component-a>" },
        // An unrelated file
        TestFile { path: "pages/index.html", content: "..." },
    ];
    let (_tmp_dir, source_dir) = setup_test_environment(&files_to_create);
    
    // Let's say component-a was the one initially modified.
    let initial_modified_files = source_dir.join("shared/reusables/component-a/component-a.html").to_string_lossy().to_string();
    let components_dir = source_dir.join("shared/reusables");

    // --- ACT ---
    let result = get_all_files_that_need_recompiling(
        &source_dir.to_string_lossy(),
        "<r-",
        ">",
        &HashSet::from([initial_modified_files.clone()]),
        &components_dir.to_string_lossy(),
    );

    // --- ASSERT ---
    // The function should detect that B depends on A, add B to the list.
    // On the next loop, it will detect A depends on B, but A is already in the list.
    // The list stops growing, and the loop terminates. The result should contain both A and B.
    let expected_files: HashSet<String> = HashSet::from([
        initial_modified_files.clone(),
        source_dir.join("shared/reusables/component-b/component-b.html").to_string_lossy().to_string(),
    ]);
    
    println!("Expected final list:\n{:?}", expected_files);
    println!("Actual final list:\n{:?}", result);
    
    assert_eq!(result, expected_files);
    println!("--- Test Passed: The function did not enter an infinite loop. ---\n");
}

// ===================================================================
// # === Test Harness for Mocking Command-Line Tools ===
// ===================================================================

/// A helper function to create a mock command-line tool (e.g., `sass`, `tsc`) for testing.
/// This is essential because we don't want our tests to fail if `sass` isn't installed
/// on the machine running the tests. This makes our tests **hermetic** (self-contained).
///
/// # Arguments
/// * `mock_dir` - The temporary directory where the mock executable will be placed.
/// * `command_name` - The name of the command to mock, e.g., "sass" or "tsc".
/// * `script_content` - The shell script code that the mock command will execute.
///
/// # Example Script Content
/// * Success case: `#!/bin/sh\necho "Mock SASS running..." >&2\ntouch "$2"\nexit 0`
///   (This script prints a message, creates the output file, and exits successfully)
/// * Failure case: `#!/bin/sh\necho "Error: SCSS syntax error" >&2\nexit 1`
///   (This script prints an error and exits with a failure code)
///
/// Keep the returned guard alive for the whole test (`let _mock = ...`), it puts PATH back when dropped.
fn setup_mock_command(mock_dir: &Path, command_name: &str, script_content: &str) -> MockCommandGuard {
    // Tests run in parallel but PATH is shared by the whole process, so the mock tests take turns.
    let lock = PATH_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    let mock_executable_path = mock_dir.join(command_name);
    fs::write(&mock_executable_path, script_content)
        .expect("Failed to write mock command script");

    // On Unix-like systems (Linux, macOS), we need to make the script executable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&mock_executable_path, fs::Permissions::from_mode(0o755))
            .expect("Failed to set mock command as executable");
    }

    // Prepend the mock directory to the system's PATH environment variable for the current process.
    // This ensures that when `Command::new("sass")` is called, our mock script is found and
    // executed instead of the real one.
    let original_path = env::var("PATH").unwrap_or_default();
    let new_path = env::join_paths(std::iter::once(mock_dir.to_path_buf()).chain(env::split_paths(&original_path)))
        .expect("Failed to build mock PATH");
    // SAFETY: only the mock tests change PATH, and they hold PATH_LOCK whilst they do.
    unsafe { env::set_var("PATH", new_path) };

    MockCommandGuard { original_path, _lock: lock }
}

static PATH_LOCK: Mutex<()> = Mutex::new(());

/// Restores the original PATH (and lets the next mock test run) once the test is done.
struct MockCommandGuard {
    original_path: String,
    _lock: MutexGuard<'static, ()>,
}

impl Drop for MockCommandGuard {
    fn drop(&mut self) {
        // SAFETY: still holding PATH_LOCK here (fields are dropped after this runs).
        unsafe { env::set_var("PATH", &self.original_path) };
    }
}

// ===================================================================
// # === Tests for `compile_scss_file` ===
// ===================================================================

// Test Case 1: Happy Path - SCSS compilation succeeds.
// Checks that if the source file exists and the (mock) sass command succeeds,
// the function returns Ok, and the output CSS file is created.
#[test]
fn test_compile_scss_success() {
    println!("\n--- Running Test: test_compile_scss_success ---");
    // --- ARRANGE ---
    // 1. Create a temporary directory for our test files and mocks.
    let tmp = tempdir().unwrap();
    // 2. Define the script for our mock `sass` command that simulates success.
    //    `$1` is the input file, `$2` is the output file in shell scripts.
    let mock_sass_script = format!("#!/bin/sh\n# This is a mock sass command\ntouch \"$2\"\nexit 0");
    let _mock = setup_mock_command(tmp.path(), "sass", &mock_sass_script);
    
    // 3. Create a dummy source SCSS file. Its content doesn't matter.
    let source_scss_file = tmp.path().join("style.scss");
    let output_css_file = tmp.path().join("dist/style.css");
    fs::write(&source_scss_file, "body { color: blue; }").unwrap();

    // --- ACT ---
    // 4. Run the function we're testing.
    let result = compile_scss_file(
        source_scss_file.to_str().unwrap(),
        output_css_file.to_str().unwrap(),
    );

    // --- ASSERT ---
    // 5. Verify the outcome.
    println!("Function result: {:?}", result);
    assert!(result.is_ok(), "Function should return Ok on success");

    println!("Checking if output file exists: {:?}", &output_css_file);
    assert!(output_css_file.exists(), "Output CSS file should be created by the mock command");
    println!("--- Test Passed ---");
}

// Test Case 2: Failure - Source SCSS file does not exist.
// Checks that the function correctly returns an Err when the input file is missing
// without attempting to run the compiler.
#[test]
fn test_compile_scss_source_not_found() {
    println!("\n--- Running Test: test_compile_scss_source_not_found ---");
    // --- ARRANGE ---
    let tmp = tempdir().unwrap();
    let non_existent_source = tmp.path().join("non_existent.scss");
    let output_css_file = tmp.path().join("dist/style.css");

    // --- ACT ---
    let result = compile_scss_file(
        non_existent_source.to_str().unwrap(),
        output_css_file.to_str().unwrap(),
    );

    // --- ASSERT ---
    println!("Function result: {:?}", result);
    assert!(result.is_err(), "Function should return Err when source is not found");
    assert!(result.unwrap_err().contains("source scss file not found"));
    
    println!("Checking that output file was not created");
    assert!(!output_css_file.exists(), "Output file should not be created if source is missing");
    println!("--- Test Passed ---");
}

// Test Case 3: Failure - `sass` command fails (returns non-zero exit code).
// Checks that the function captures the failure from the external command and reports it.
#[test]
fn test_compile_scss_command_fails() {
    println!("\n--- Running Test: test_compile_scss_command_fails ---");
    // --- ARRANGE ---
    let tmp = tempdir().unwrap();
    // This mock `sass` command simulates a syntax error by exiting with code 1.
    let mock_sass_script = "#!/bin/sh\necho 'Error: Invalid CSS after \"body...\": expected \"{\", was \"}\"' >&2\nexit 1";
    let _mock = setup_mock_command(tmp.path(), "sass", &mock_sass_script);
    
    let source_scss_file = tmp.path().join("style.scss");
    let output_css_file = tmp.path().join("dist/style.css");
    fs::write(&source_scss_file, "body { color: blue; }").unwrap();

    // --- ACT ---
    let result = compile_scss_file(
        source_scss_file.to_str().unwrap(),
        output_css_file.to_str().unwrap(),
    );

    // --- ASSERT ---
    println!("Function result: {:?}", result);
    assert!(result.is_err(), "Function should return Err on compilation failure");
    let error_message = result.unwrap_err();
    println!("Error message: {}", error_message);
    assert!(error_message.contains("error compiling scss file"));
    assert!(error_message.contains("Invalid CSS")); // Check that stderr from the mock is captured.

    println!("--- Test Passed ---");
}


// ===================================================================
// # === Tests for `compile_typescript_file` ===
// ===================================================================

// Test Case 4: Happy Path - TypeScript compilation succeeds.
// Checks that on success, the function returns Ok, the JS output is created,
// AND the original TS source file is kept (shared/code/*.ts files get inlined from the output directory).
#[test]
fn test_compile_typescript_success() {
    println!("\n--- Running Test: test_compile_typescript_success ---");
    // --- ARRANGE ---
    let tmp = tempdir().unwrap();
    // Mock `tsc` that creates the output file (called as: tsc --target es2015 <source> --outDir <dir>, so the output is $5/<source name>.js)
    let mock_tsc_script = "#!/bin/sh\n# This is a mock tsc command\ntouch \"$5/$(basename \"$3\" .ts).js\"\nexit 0";
    let _mock = setup_mock_command(tmp.path(), "tsc", &mock_tsc_script);
    
    let source_ts_file = tmp.path().join("app.ts");
    let output_js_file = tmp.path().join("dist/app.js");
    fs::write(&source_ts_file, "const x: number = 1;").unwrap();

    // --- ACT ---
    let result = compile_typescript_file(
        source_ts_file.to_str().unwrap(),
        output_js_file.to_str().unwrap(),
    );

    // --- ASSERT ---
    println!("Function result: {:?}", result);
    assert!(result.is_ok(), "Function should return Ok on success");

    println!("Checking if output file exists: {:?}", &output_js_file);
    assert!(output_js_file.exists(), "Output JS file should be created");

    println!("Checking if source file was kept: {:?}", &source_ts_file);
    assert!(source_ts_file.exists(), "Source TS file should be kept on success");
    println!("--- Test Passed ---");
}


// Test Case 5: Failure - `tsc` command fails.
// Checks that if the compiler fails, the function returns an Err, and critically,
// the original source file is NOT deleted.
#[test]
fn test_compile_typescript_command_fails() {
    println!("\n--- Running Test: test_compile_typescript_command_fails ---");
    // --- ARRANGE ---
    let tmp = tempdir().unwrap();
    // Mock `tsc` that simulates a type error and exits with code 1.
    let mock_tsc_script = format!("#!/bin/sh\necho \"app.ts(1,7): error TS2322: Type 'string' is not assignable to type 'number'.\" >&2\nexit 1");
    let _mock = setup_mock_command(tmp.path(), "tsc", &mock_tsc_script);
    
    let source_ts_file = tmp.path().join("app.ts");
    let output_js_file = tmp.path().join("dist/app.js");
    fs::write(&source_ts_file, "const x: number = 'hello';").unwrap();
    
    // --- ACT ---
    let result = compile_typescript_file(
        source_ts_file.to_str().unwrap(),
        output_js_file.to_str().unwrap(),
    );

    // --- ASSERT ---
    println!("Function result: {:?}", result);
    assert!(result.is_err(), "Function should return Err on compilation failure");

    println!("Checking if source file STILL exists: {:?}", &source_ts_file);
    assert!(source_ts_file.exists(), "Source TS file should NOT be deleted on failure");
    
    println!("Checking that output file was NOT created");
    assert!(!output_js_file.exists(), "Output JS file should not be created on failure");
    println!("--- Test Passed ---");
}

//



// ============================================================
// GENERATING TRANSLATION FILES
// ============================================================

// use super::*;
// 
// #[test]
// fn test_translation_lookup_functionality() {
    // let mut test_translation_cache: TranslationCacheMap = HashMap::new();
    // let mut english_language_map: HashMap<String, HashMap<String, String>> = HashMap::new();
    // let mut index_page_variables: HashMap<String, String> = HashMap::new();
    // index_page_variables.insert("title".to_string(), "Home Page".to_string());
    // english_language_map.insert("index".to_string(), index_page_variables);
    // test_translation_cache.insert("en".to_string(), english_language_map);
    // 
    // assert_eq!(
        // get_translation_for_language_page_variable(&test_translation_cache, "en", "index", "title"), 
        // Some("Home Page")
    // );
    // assert_eq!(
        // get_translation_for_language_page_variable(&test_translation_cache, "en", "index", "missing_variable"), 
        // None
    // );
// }
// 
// #[test]
// fn test_extract_translation_keys_from_html() {
    // let test_html_content: &str = r#"
        // <div>TEXT=pages.index.title</div>
        // <p>Some other static content</p>
        // <div>TEXT=pages.about.header</div>
    // "#;
    // 
    // let extracted_keys: Vec<String> = extract_translation_keys_from_html_content(test_html_content);
    // assert_eq!(extracted_keys, vec!["pages.index.title", "pages.about.header"]);
// }
// 
// #[test]
// fn test_html_content_processing_with_translations() {
    // let original_html_content: &str = r#"
        // <html>
            // <body>
                // <div>TEXT=pages.index.title</div>
                // <div>TEXT=pages.index.subtitle</div>
            // </body>
        // </html>
    // "#;
    // 
    // let mut test_translation_cache: TranslationCacheMap = HashMap::new();
    // let mut english_language_map: HashMap<String, HashMap<String, String>> = HashMap::new();
    // let mut index_page_variables: HashMap<String, String> = HashMap::new();
    // index_page_variables.insert("title".to_string(), "Welcome Home".to_string());
    // index_page_variables.insert("subtitle".to_string(), "Your Home Page".to_string());
    // english_language_map.insert("index".to_string(), index_page_variables);
    // test_translation_cache.insert("en".to_string(), english_language_map);
    // 
    // let translation_placeholder_regex: Regex = Regex::new(r"<div>TEXT=([a-zA-Z0-9_\.]+)</div>")
        // .expect("Failed to compile regex");
    // 
    // let processed_html_content: String = translation_placeholder_regex.replace_all(
        // original_html_content, 
        // |regex_captures: &regex::Captures| {
            // let full_translation_key: &str = &regex_captures[1];
            // if let Some((page_name, variable_name)) = full_translation_key.split_once('.') {
                // if let Some(translated_text) = get_translation_for_language_page_variable(
                    // &test_translation_cache, 
                    // "en", 
                    // page_name, 
                    // variable_name
                // ) {
                    // format!("<div>{}</div>", translated_text)
                // } else {
                    // format!("<div>{}</div>", full_translation_key)
                // }
            // } else {
                // format!("<div>TEXT={}</div>", full_translation_key)
            // }
        // }
    // ).to_string();
    // 
    // assert!(processed_html_content.contains("<div>Welcome Home</div>"));
    // assert!(processed_html_content.contains("<div>Your Home Page</div>"));
// }
// 
