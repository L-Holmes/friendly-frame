use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::io::{self, Write};
use std::process::Command;
use anyhow::Result;
use regex::{Regex};
use std::collections::HashSet;
use walkdir::WalkDir;

// === DETERMINING WHICH FILES TO UPDATE ===

/// Function to read the last compile time from the hidden file
/// usage: Add to the top of the compilation script:
/// let time = read_last_compile_time(last_compile_time_file);
pub fn read_last_compile_time(last_compile_time_file: &str) -> u64 {
    if Path::new(last_compile_time_file).exists() {
        let content: String = fs::read_to_string(last_compile_time_file).unwrap_or_else(|_| "0".to_string());
        let parsed_time: u64 = content.trim().parse::<u64>().unwrap_or(0);
        parsed_time
    } else {
        0
    }
}

/// Function to update the last compile time in the hidden file
/// usage: Add to the end of the compilation script:
/// update_last_compile_time(time, last_compile_time_file).unwrap();
pub fn update_last_compile_time(the_time: u64, last_compile_time_file: &str) -> io::Result<()> {
    let mut file: fs::File = fs::File::create(last_compile_time_file)?;
    writeln!(file, "{}", the_time)?;
    Ok(())
}

/// Function to get all files in the source directory
pub fn files_in_source_dir(dir: &str) -> Result<Vec<PathBuf>> {
    let output = Command::new("find").arg(dir).arg("-type").arg("f").output()?;
    let files_vec: Vec<PathBuf> = String::from_utf8_lossy(&output.stdout).lines().map(|line| PathBuf::from(line)).collect();
    Ok(files_vec)
}

/// Function to filter files by modification time and return only the newer ones.
///
/// # Usage
/// `let newly_modified_files = only_modify_time_newer_than_last_compile_time(&all_files, last_compile_time)?;`
///
/// # Parameters
/// * `all_files`: A list of file paths to check. Example: `&vec![PathBuf::from("src/main.rs")]`
/// * `last_compile_time`: The timestamp to compare file modification times against. 
pub fn only_modify_time_newer_than_last_compile_time( all_files: &Vec<PathBuf>, last_compile_time: u64) -> Result<Vec<PathBuf>> {
    let mut newer_files: Vec<PathBuf> = Vec::new();

    for file_path in all_files {
        if file_path.is_file() {
            // Get file metadata, which includes modification time.
            // The '?' operator will propagate any errors, like if the file doesn't exist.
            let metadata: fs::Metadata = fs::metadata(&file_path)?;
            let file_mod_time = metadata.modified()?.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

            // If file modification time is newer than last compile time, add to the list
            if file_mod_time > last_compile_time {
                newer_files.push(file_path.clone());
            }
        }
    }

    Ok(newer_files)
}

// === FILE PATHS ===

/// Calculates a relative path of `../` segments based on the directory depth of the input path.
/// This is a direct translation of the original shell script's logic, which effectively counts the
/// number of directory segments in a path and returns `(count - 2)` instances of `../`.
pub fn get_relative_path(filepath: &Path) -> String {
    // Count the number of normal path segments (e.g., 'a', 'b', 'c'). This ignores things like './' or '/'.
    // The shell script's `sed` command effectively did this by counting non-empty segments ending in a slash.
    let segment_count = filepath.components().filter(|c| matches!(c, std::path::Component::Normal(_))).count();

    // The original script's logic (`#../../`) removes two `../` segments.
    // We use saturating_sub to ensure the result is never negative; if count is 2 or less, the result is 0.
    let num_of_dots = segment_count.saturating_sub(2);

    if num_of_dots > 0 {
        // Repeat '../' for the calculated number of times.
        "../".repeat(num_of_dots)
    } else {
        // If there are 2 or fewer segments, the result is an empty string.
        String::new()
    }
}


// === REPLACING COMPONENT PLACEHOLDERS ===

/// Searches a file for a placeholder and replaces it with a calculated relative path.
/// This is a direct port of the `replace_root_placeholder_with_relative_path_new` shell function.
/// @param search_regex = a regex that will match '<root>'. E.g. If user has imported an image: <root>/shared/images/img1.png
/// @param root_placeholder = the actual placeholder. (i.e. '<root>')
/// @param target_file = the path to the target file (either relative or absolute if it starts with '/')
/// @what this does => Replaces '<root>' with the '../../..' etc. string required to get from that file to the root of the project, if using 'cd' in the terminal
pub fn replace_root_placeholder_with_relative_path_new( search_text: &str, root_placeholder: &str, target_file: &Path,) -> Result<()> {
    if !target_file.is_file() {
        println!("-> Warning! the target file: {} is not a file...", target_file.display());
        return Ok(()); // Silently skip if the file doesn't exist.
    }

    // Read raw bytes
    let bytes = fs::read(target_file)?;
    let content = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => {
            // Not valid UTF-8 → probably binary (like .ico)
            return Ok(());
        }
    };

    // Return early if no match (grep -F style)
    if !content.contains(search_text) {
        return Ok(());
    }

    // Append dummy to ensure same segment behavior as original
    let path_with_slash = target_file.join("");
    let sub_text = get_relative_path(&path_with_slash);

    let final_placeholder = format!("{}/", root_placeholder);
    let new_content = content.replace(&final_placeholder, &sub_text);
    fs::write(target_file, new_content)?;

    Ok(())
}


// === JAVASCRIPT COMPONENTS ===
pub fn add_reusable_javascript_components( javascript_file: &Path, root_folder: &Path, shared_code_folder: &str, re_start: &str, re_end: &str,) -> Result<()> {
    if !javascript_file.is_file() {
        return Ok(());
    }

    let mut content = fs::read_to_string(javascript_file)?;
    let re = Regex::new(&format!(r"(?m){}(.*?){}", regex::escape(re_start), regex::escape(re_end)))?;

    let captures: Vec<(String, String)> = re.captures_iter(&content)
        .map(|cap| (cap[0].to_string(), cap[1].to_string()))
        .collect();

    for (full_tag, component_name) in captures {
        let component_path = root_folder.join("shared").join(shared_code_folder).join(format!("{}.ts", component_name));

        if !component_path.is_file() {
            eprintln!("ERROR!!!!!!! Associated file ({}) for component '{}' not found.", component_path.display(), full_tag);
            report_missing_file_error(&component_path, &component_name, &full_tag);
            continue;
        }

        let component_file_content = fs::read_to_string(component_path)?;
        content = content.replace(&full_tag, &component_file_content);
    }

    fs::write(javascript_file, content)?;
    Ok(())
}

fn report_missing_file_error(file_path: &std::path::Path, component_name: &str, full_tag: &str) {

    if let Some(parent_dir) = file_path.parent() {
        if parent_dir.exists() {
            eprintln!("ERROR!!!!!!! Associated file ({}) for component '{}' not found. ({})", 
                     file_path.display(), component_name, full_tag);
            
            // List contents of parent directory
            if let Ok(entries) = std::fs::read_dir(parent_dir) {
                eprintln!("Contents of parent directory {}:", parent_dir.display());
                for entry in entries.flatten() {
                    eprintln!("  {}", entry.file_name().to_string_lossy());
                }
            }
        } else {
            eprintln!("ERROR!!!!!!! Parent dir {} doesn't exist either!", parent_dir.display());
        }
    }


    println!("/shared/code");
    match std::fs::read_dir("actual-website-do-not-edit/shared/code") { Ok(entries) => entries.flatten().for_each(|e| println!("{}", e.file_name().to_string_lossy())), Err(e) => println!("Error: {}", e) }
    println!("/shared/");
    match std::fs::read_dir("actual-website-do-not-edit/shared") { Ok(entries) => entries.flatten().for_each(|e| println!("{}", e.file_name().to_string_lossy())), Err(e) => println!("Error: {}", e) }
    println!("/");
    match std::fs::read_dir("actual-website-do-not-edit") { Ok(entries) => entries.flatten().for_each(|e| println!("{}", e.file_name().to_string_lossy())), Err(e) => println!("Error: {}", e) }
    println!(".");
    match std::fs::read_dir(".") { Ok(entries) => entries.flatten().for_each(|e| println!("{}", e.file_name().to_string_lossy())), Err(e) => println!("Error: {}", e) }
}


// === HTML COMPONENTS ===

/// Recursively finds and replaces HTML component tags with their corresponding HTML, CSS, and JS.
pub fn replace_html_component_placeholders( html_file: &Path, src_directory: &Path, re_start: &str, re_end: &str, root_placeholder: &str, template_prefix: &str, none_prefix: &str, re_param_s: &str, re_param_e: &str, display_name_placeholder_start: &str, display_name_placeholder_end: &str) -> Result<()> {
    if !html_file.is_file() { return Ok(()); }

    let mut content = fs::read_to_string(html_file)?;
    let mut collected_params = std::collections::HashMap::new(); // This will store the parameters across iterations.
    
    let component_regex = Regex::new(&format!(r"{}([a-zA-Z0-9_-]+)([^>]*){}", regex::escape(re_start), regex::escape(re_end)))?; // Regex to find a component tag, its name, and the full parameters string.
    let param_regex = Regex::new(r#"(?P<key>[a-zA-Z0-9_-]+)=(?P<value>'[^']*'|"[^"]*"|[^\s'"]+)"#)?; // Regex to find individual key="value" or key='value' pairs within the parameters string.

    // It keeps processing the file until no more component tags can be found.
    loop {
        let caps = match component_regex.captures(&content) {
            Some(c) => c, // If we find a match, process it
            None => break, // No match, so exit the loop
        };

        // By processing one match at a time, we faithfully replicate the original script's behavior.
        let full_tag = caps.get(0).unwrap().as_str().to_string();
        let component_name = caps.get(1).unwrap().as_str().to_string();
        let params_str = caps.get(2).unwrap().as_str().to_string();

        // Collect parameters into the hashmap (we just keep adding to the params loop each time, so we can carry over params when handling nested components) (we just keep adding to the params loop each time, so we can carry over params when handling nested components)
        for p_caps in param_regex.captures_iter(&params_str) {
            let key = &p_caps["key"];
            let mut value = p_caps["value"].to_string();

            // Remove quotes if present
            if value.starts_with('"') && value.ends_with('"') || value.starts_with('\'') && value.ends_with('\'') {
                value = value[1..value.len()-1].to_string();
            }

            collected_params.insert(key.to_string(), value.clone());
        }

        // This is the beginning of the merged `add_html_component` logic.
        let associated_folder = PathBuf::from("shared/reusables").join(&component_name);
        let associated_folder_in_src = src_directory.join(&associated_folder);

        if !associated_folder_in_src.is_dir() {
            eprintln!("ERROR! Associated folder for component '{}' not found. Expected a folder: {}", component_name, associated_folder_in_src.display());
            content = content.replace(&full_tag, &format!("<!-- ERROR: Component '{}' directory not found. -->", component_name));


            // -- verbosely output parent directory for debugging --
            if let Some(parent) = associated_folder_in_src.parent() {
                if parent.is_dir() {
                    eprintln!("Contents of parent folder '{}':", parent.display());
                    match fs::read_dir(parent) {
                        Ok(entries) => {
                            for entry in entries.flatten() {
                                eprintln!(" - {}", entry.file_name().to_string_lossy());
                            }
                        }
                        Err(err) => {
                            eprintln!("Could not read contents of parent folder: {}", err);
                        }
                    }
                } else {
                    eprintln!("Parent folder '{}' does not exist!", parent.display());
                }
            } else {
                eprintln!("No parent folder found for '{}'", associated_folder_in_src.display());
            }
            // -----------------------------------------------------
            continue;
        }

        let associated_html_in_src = associated_folder_in_src.join(format!("{}.html", component_name));
        let html_to_insert = if associated_html_in_src.is_file() {
            fs::read_to_string(&associated_html_in_src)?
        } else {
            eprintln!("ERROR! Associated HTML file for component '{}' not found", component_name);
            format!("<!-- ERROR: HTML file for Component '{}' not found. -->", component_name)
        };
        
        // This corresponds to `sed -i "s|$component_tag|$html_contents|g"`.
        // We do the final replacement at the end, after all parameters are processed.
        let mut processed_html = html_to_insert.clone();

        // --------------------------------------------------------------------------------------------------
        // -- Process parameters --
        // --------------------------------------------------------------------------------------------------
        // What this does, for each found parameter ('x=y') in the component tag:
        // 1) If the param is '@none', it makes the sub text ''
        // 2) If the param is '@template:...' it makes the sub text '<r-templatename>'
        // 3) If the param is 'hello' (i.e. any none '@' text), it makes the sub text 'hello' (i.e. the text)
        // 4) It inserts that text at the location of the {placeholder} in the component code.
        for (key, value) in &collected_params {
            let mut value = value.clone(); // Get the value from the hashmap
            //e.g. key =  filename OR title OR innerComponent
            //e.g. value = "hi" OR @template:side-bar OR @none

            // Handle `@template:` prefix.
            if value.starts_with(template_prefix) {
                let template_content = value.strip_prefix(template_prefix).unwrap(); //e.g. side-bar
                value = format!("{}{}{}", re_start, template_content, re_end);  //e.g. "<r-side-bar>" (i.e. it adds it as a component, so on the next cycle, it will be converted).
            } 
            // Handle `@none` prefix.
            else if value.starts_with(none_prefix) {
                value = String::new(); // e.g. ""
            }

            // This corresponds to `sed -i "s|$re_param_s$param_variable_name$re_param_e|$param_variable_value|g"`
            // i.e. What this does, is it looks in the code *of the component* (e.g. top-bar.html), for where the user has defined where the parameter should be inserted
            // e.g. They may have added '{title}' to represent the location where they would like the value of 'title' to be inserted.
            let placeholder = format!("{}{}{}", re_param_s, key, re_param_e);
            processed_html = processed_html.replace(&placeholder, &value);

            // Do the same as the above, but for string representations of params.
            //  -> e.g. if the 'key' is 'animals.mammals.blue_whale.jpg'
            //  -> Instead of subbing in 'animals.mammals.blue_whale.jpg', it will sub in: 'Blue whale'
            let extracted_name: String = extract_display_name_from_filepath(&value);
            let placeholder2 = format!("{}{}{}", display_name_placeholder_start, key, display_name_placeholder_end);

            // =============== DEBUGGING =========================

            // // Count matches
            // let match_count = processed_html.matches(&placeholder2).count();
// 
            // println!("=== Placeholder Replacement Debug ===");
            // println!("Placeholder: '{}'", placeholder2);
            // println!("Replacement: '{}'", extracted_name);
            // println!("Match count: {}", match_count);
// 
            // if match_count == 0 {
                // println!("⚠️  No matches found for placeholder: '{}'", placeholder2);
                // println!("Text content (first 500 chars):\n{}", 
                    // &processed_html.chars().take(500).collect::<String>());
// 
                // // Show all unique substrings that might be similar
                // println!("\nSearching for similar patterns containing '{}':", key);
                // for line in processed_html.lines() {
                    // if line.contains(key) {
                        // println!("  Found in line: {}", line.trim());
                    // }
                // }
            // } else {
                // println!("✓ Found {} occurrence(s)", match_count);
// 
                // // Show context around each match (before replacement)
                // println!("\nContext before replacement:");
                // let mut start = 0;
                // while let Some(pos) = processed_html[start..].find(&placeholder2) {
                    // let abs_pos = start + pos;
                    // let context_start = abs_pos.saturating_sub(50);
                    // let context_end = (abs_pos + placeholder2.len() + 50).min(processed_html.len());
// 
                    // println!("  ...{}...", &processed_html[context_start..context_end]);
                    // start = abs_pos + placeholder2.len();
                // }
            // }
            // =============== DEBUGGING END =========================

            // Perform the replacement
            processed_html = processed_html.replace(&placeholder2, &extracted_name);

            // =============== DEBUGGING =========================

            // // Show context after replacement (if there were matches)
            // if match_count > 0 {
                // println!("\nContext after replacement:");
                // let mut start = 0;
                // while let Some(pos) = processed_html[start..].find(&extracted_name) {
                    // let abs_pos = start + pos;
                    // let context_start = abs_pos.saturating_sub(50);
                    // let context_end = (abs_pos + extracted_name.len() + 50).min(processed_html.len());
                    // 
                    // println!("  ...{}...", &processed_html[context_start..context_end]);
                    // start = abs_pos + extracted_name.len();
                // }
            // }
// 
            // println!("=====================================\n");
            // =============== DEBUGGING END =========================




        }
        // --------------------------------------------------------------------------------------------------

        // --------------------------------------------------------------------------------------------------
        // This section handles adding CSS and JS asset links, ported from `add_html_component`.
        // --------------------------------------------------------------------------------------------------
        if let Some(head_pos) = content.find("</head>") {
            let mut head_inserts = String::new();
            
            // Check for the UNCOMPILED .scss file.
            let associated_scss_in_src = associated_folder_in_src.join(format!("{}.scss", &component_name));
            if associated_scss_in_src.exists() {
                // Create a link to the final COMPILED .css file.
                let css_path = format!("{}/{}.css", associated_folder.to_string_lossy(), component_name);
                let css_link = format!("<link rel=\"stylesheet\" href=\"{}/{}\">\n", root_placeholder, css_path);
                if !content.contains(&css_link) { head_inserts.push_str(&css_link); }
            }

            // Check for the UNCOMPILED .ts file.
            let associated_ts_in_src = associated_folder_in_src.join(format!("{}.ts", &component_name));
            if associated_ts_in_src.exists() {
                // Create a link to the final COMPILED .js file.
                let js_path = format!("{}/{}.js", associated_folder.to_string_lossy(), component_name);
                let js_link = format!("<script defer src=\"{}/{}\"></script>\n", root_placeholder, js_path);
                if !content.contains(&js_link) { head_inserts.push_str(&js_link); }
            }
            content.insert_str(head_pos, &head_inserts);
        }
        // --------------------------------------------------------------------------------------------------

        // Now, perform the final replacement of the tag with its fully processed HTML.
        content = content.replace(&full_tag, &processed_html);
    }

    fs::write(html_file, content)?;
    Ok(())
}


/// Extracts a human-readable display name from a dot-separated filepath.
/// 
/// This function takes a filepath like 'animals.mammals.blue_whale.jpg' and extracts
/// the penultimate (second-to-last) segment, converts underscores to spaces, and 
/// formats it in title case.
/// 
/// # Arguments
/// * `filepath` - A string slice representing a dot-separated filepath
/// 
/// # Returns
/// A `String` containing the formatted display name
/// 
/// # Examples
/// ```ignore
/// let name = extract_display_name_from_filepath("animals.mammals.blue_whale.jpg");
/// assert_eq!(name, "Blue Whale");
/// 
/// let name = extract_display_name_from_filepath("single_word");
/// assert_eq!(name, "Single Word");
/// ```
fn extract_display_name_from_filepath(filepath: &str) -> String {
    let parts: Vec<&str> = filepath.split('.').collect();
    
    let target_part: &str = if parts.len() >= 2 {
        // Get the penultimate (second to last) part
        parts[parts.len() - 2]
    } else {
        // If there's only one part or no parts, use the entire filepath
        filepath
    };
    
    // Replace underscores with spaces and convert to title case
    target_part
        .replace('_', " ")
        .split_whitespace()
        .map(|word: &str| -> String {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let uppercase_first: String = first.to_uppercase().collect();
                    let lowercase_rest: String = chars.as_str().to_lowercase();
                    uppercase_first + &lowercase_rest
                }
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}



// === compiling languages ===

/// Compiles a single SCSS file to a CSS file using the `sass` command-line tool.
///
/// # Usage
///
/// ```no_run
/// # use website_compiler::components::compile_scss_file;
/// // Corresponds to shell: compile_scss_file "$scss_source" "$scss_output"
/// match compile_scss_file("path/to/style.scss", "path/to/style.css") {
///     Ok(_) => println!("Compilation successful!"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
///
/// # Arguments
///
/// * `scss_source` - The path to the source SCSS file.
/// * `css_output` - The path where the final CSS file will be saved.
///
/// # Returns
///
/// * `Ok(())` if compilation is successful.
/// * `Err(String)` with an error message if the source file is not found or if the `sass` command fails.
pub fn compile_scss_file(scss_source: &str, css_output: &str) -> Result<(), String> {
    let source_path = Path::new(scss_source);
    
    // check if the source file exists
    if !source_path.exists() {
        let warning_msg = format!("warning: source scss file not found: {}", scss_source);
        println!("{}", warning_msg);
        // In shell, this was `return 1`. In Rust, we return an Err.
        return Err(warning_msg);
    }
    
    println!("compiling scss: {} -> {}", scss_source, css_output);
    
    // create directory for the css file if it doesn't exist
    // `Path::parent` returns the directory containing the file.
    if let Some(parent_dir) = Path::new(css_output).parent() {
        fs::create_dir_all(parent_dir).map_err(|e| format!("failed to create output directory: {}", e))?;
    }
    
    // compile scss to css by calling the external `sass` command
    let output = Command::new("sass").arg(scss_source).arg(css_output).output().map_err(|e| format!("failed to execute sass command. Is `sass` installed and in your PATH? Error: {}", e))?;

    // Check if the command executed successfully.
    if !output.status.success() {
        let error_msg = format!( "error compiling scss file: {}\nSass Error Output:\n{}", scss_source, String::from_utf8_lossy(&output.stderr));
        eprintln!("{}", error_msg);
        return Err(error_msg);
    }
    
    Ok(())
}


/// Compiles a single TypeScript file to a JavaScript file using the `tsc` command-line tool.
/// Upon successful compilation, the original TypeScript source file is deleted.
///
/// # Usage
///
/// ```no_run
/// # use website_compiler::components::compile_typescript_file;
/// // Corresponds to shell: compile_typescript_file "$ts_source" "$js_output"
/// match compile_typescript_file("path/to/app.ts", "path/to/app.js") {
///     Ok(_) => println!("Compilation successful!"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
///
/// # Arguments
///
/// * `ts_source` - The path to the source TypeScript file.
/// * `js_output` - The path where the final JavaScript file will be saved.
///
/// # Returns
///
/// * `Ok(())` if compilation and file removal are successful.
/// * `Err(String)` with an error message if any step fails.
pub fn compile_typescript_file(ts_source: &str, js_output: &str) -> Result<(), String> {
    let source_path = Path::new(ts_source);

    // check if the source file exists
    if !source_path.is_file() {
        let warning_msg = format!("warning: source typescript file not found: {}", ts_source);
        println!("{}", warning_msg);
        return Err(warning_msg);
    }

    println!("compiling and minifying TypeScript: {} -> {}", ts_source, js_output);

    // create directory for the js file if it doesn't exist
    if let Some(parent_dir) = Path::new(js_output).parent() {
        fs::create_dir_all(parent_dir).map_err(|e| format!("failed to create output directory: {}", e))?;
    }

    // compile and minify with tsc
    // (uses --outDir, since newer versions of typescript (7+) removed --outFile)
    let out_dir = Path::new(js_output).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let output = Command::new("tsc").arg("--target").arg("es2015").arg(ts_source).arg("--outDir").arg(out_dir).output().map_err(|e| format!("failed to execute tsc command. Is `typescript` installed and in your PATH? Error: {}", e))?;

    if !output.status.success() {
        // Capture both stdout and stderr for complete error information
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        let mut error_output = String::new();
        if !stdout_str.trim().is_empty() {
            error_output.push_str(&stdout_str);
        }
        if !stderr_str.trim().is_empty() {
            if !error_output.is_empty() {
                error_output.push('\n');
            }
            error_output.push_str(&stderr_str);
        }

        let error_msg = format!(
            "error compiling typescript file: {}\nTSC Error Output:\n{}", 
            ts_source, 
            error_output
        );
        eprintln!("{}", error_msg);
        return Err(error_msg);
    }

    // tsc names the output after the source file (e.g. app.ts -> app.js), so move it if a different name was asked for
    let emitted_js = out_dir.join(source_path.with_extension("js").file_name().unwrap_or_default());
    if emitted_js != Path::new(js_output) {
        fs::rename(&emitted_js, js_output).map_err(|e| format!("compiled, but failed to move {} to {}: {}", emitted_js.display(), js_output, e))?;
    }

    // remove the original .ts file
    // The `?` operator here will convert a file removal error into our `Err(String)`.
    // fs::remove_file(ts_source).map_err(|e| format!("successfully compiled but failed to remove source file {}: {}", ts_source, e))?; // TODO remove this? because of the js component import error!?!?!?!?

    // Corresponds to shell `return 0`
    Ok(())
}


// === GETTING THE LIST OF MODIFIED COMPONENTS IN ORDER ===

// ================================================================
// CASCADING FILE THAT NEED TO BE UPDATED SINCE THE REUSABLE COMPONENTS THEY USE HAVE BEEN UPDATED
// ==================================================================
// * For each changed component:
// * -> components that use that component = _get_all_components_that_use_that_component()
// * add each of those components to the list of changed components, if they haven't been added already, so that they also get passed to the compileAll() function.
// * So essentially, it just adds components that reference changed components to the newly_modified_files list, if they haven't been added already.

/// Finds all files that need to be recompiled based on an initial list of modified files.
/// It recursively finds files that use modified components.
///
/// # Arguments
///
/// * `source_dir` - e.g. "/home/john/documents/proj/edit-me"
/// * `re_start` - e.g. "<r-"
/// * `re_end` - e.g. ">"
/// * `all_new_files` - A newline-separated list of paths of all files that have changed.
///   e.g.:
///     edit-me/shared/reusables/bottom-bar/bottom-bar.html
///     edit-me/shared/reusables/category-filter/category-filter-README.md
///     ...
/// * `components_dir` - Path to the reusables dir e.g. "edit-me/shared/reusables"
///
/// # Returns
///
/// The list of all_new_files, with the html files that import changed components added as well.
/// e.g.: Lets imagine that the 'index.html' file and the 'contact.html' file use the 'top-bar' component;
///     edit-me/shared/reusables/bottom-bar/bottom-bar.html
///     ...
///     edit-me/shared/reusables/top-bar/top-bar.html
///     edit-me/pages/index/index.html                                  <<<<<<<<<<<------------- new!
///     edit-me/pages/contact/contact.html                              <<<<<<<<<<<------------- new!
///
/// Essentially:
/// - Takes a list of html files
/// - Looks for any html files which import those html files (i.e. assuming the original html was a component), and adds those to the list
/// - Keeps doing this until its added all files which are linked 
// Usage: get_all_files_that_need_recompiling "$SOURCE_DIR" "$RE_START" "$RE_END" "$newly_modified_files" "$COMPONENTS_DIR" "all_files_that_are_to_be_compiled"
pub fn get_all_files_that_need_recompiling( source_dir: &str, re_start: &str, re_end: &str, all_new_files: &HashSet<String>, components_dir: &str,) -> HashSet<String> {
    // TODO refactor to be like:
    // all_html_need_recompiling = ...
    // other_html_files_which_import_any_files_to_be_recompiled = until(notChanged(lenListBefore, lenListAfter) do: allHtmlFiles | doesImportFileBeingRecompiled(all_html_need_recompiling)
    // final_list = all_html_need_recompiling + other_html_files_which_import_any_files_to_be_recompiled

    // TODO this function literally only adds html files that use reusable html files... 
    //      refactor with lots of nested private functions to clearly show this 
    
    // allHtmlFiles = ...
    // allEdittedHtmlFiles = ...
    // ** a descendent = when you follow up the import tree, you eventually find the target file **
    // def getDescendents:
        // until(NoMoreNewImportersAdded){
        //     htmlFilesThatImportEdittedFiles = allHtmlFiles | thatImportAnyOf($allEdittedHtmlFiles)
        // }
    // finalList = allEdittedHtmlFiles + htmlFilesThatImportEdittedFiles

    // Create a regex to find component tags. e.g., <r-.*?>
    let tag_regex = Regex::new(&format!("{}{}{}", regex::escape(re_start), r"(.*?)", regex::escape(re_end))).expect("Failed to create regex.");

    println!("getting all files that need recompiling; splitting the new files into components and other");

    // === Step 1: Load the current modified list into a variable (HashSet for efficiency) ===
    // Using a HashSet automatically handles duplicates and provides fast lookups.
    let mut current_modified: HashSet<String> = all_new_files.clone();
    println!("1) current modified: {:?}", current_modified);

    // ==========================================================
    // 0) Are any of the items in the all_new_files list reusable *html* components?
    //    If not, just end!
    let mut modified_html_components: HashSet<String> = current_modified.iter().filter(|path| path.contains(components_dir)).cloned().collect();

    // ==========================================================

    if !modified_html_components.is_empty() {
        // === Get a list of all html files in the project ===
        let walker: walkdir::IntoIter = WalkDir::new(source_dir).into_iter();
        let html_files: Vec<walkdir::DirEntry> = walker.filter_map(Result::ok).filter(|e| e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "html")).collect();

        // === Step 2: Start an infinite loop to scan and expand the list ===
        println!("step 2");
        loop {
            let before_count:usize = current_modified.len();

            // === Step 3: Loop through all HTML files in the source directory ===
            for entry in &html_files {
                let html_file_path:&Path = entry.path();
                let html_file_str:String = html_file_path.to_string_lossy().to_string();

                println!("\t\t\tdoing step 3:   checking the html file: {}", html_file_str);

                // === Step 4: Skip if it's already in the modified list ===
                if current_modified.contains(&html_file_str) {
                     println!("\t\t.. the html file: {} is contained in the current modified list.", html_file_str);
                     continue;
                }
                
                println!("\tdoing step 4:");

                // So at this point, we have a html file that isn't in the list, which might contain a reference to one of the items in the list (if so, this html needs adding to that list as well!)
                // i.e. this html file may import and use on the reusable html components
                // === Step 5: Extract all tags that match the reusable component pattern (e.g. <r-...>) ===

                let content : String = match std::fs::read_to_string(html_file_path) {
                    Ok(c) => c,            // `content` is bound to this string
                    Err(_) => continue,    // skip to next loop iteration
                };
                for cap in tag_regex.captures_iter(&content) {
                    let component_tag = &cap[0]; // The full match, e.g., "<r-top-bar>"
                    println!("\t\t!!!!!! step 5: found a tag: {}", component_tag);

                    // === GETTING THE NAME OF THAT COMPONENT ===
                    // Strip surrounding whitespace from the tag
                    // The regex capture group already gives us the inner content.
                    let component_tag_contents : &str = &cap[1];
                    println!("  ....(...) In the file {}, just found a tag... '{}'", html_file_str, component_tag);

                    // get the component name, which should be the first word
                    let associated_reusable_name : &str = component_tag_contents.split_whitespace().next().unwrap_or("");
                    if associated_reusable_name.is_empty() {
                        continue;
                    }

                    // === GET THE EXPECTED PATH WITHIN THE REUSABLES FOLDER OF THAT COMPONENT ===
                    let associated_reusable_dir_path : String = Path::new(components_dir).join(associated_reusable_name).join(format!("{}.html", associated_reusable_name)).to_string_lossy().to_string();

                    println!("\t\tasoc reus dir: {}", associated_reusable_dir_path);
                    
                    // === Step 6: If any component in the modified list matches this one, add this HTML file ===
                    //             i.e. The file being added *must* be a *html* file, which imports one of the reusable html components which has been modified!
                    println!("6) ??? checking whether the current modified: {:?} contains the assoc reus...", current_modified);
                    if modified_html_components.contains(&associated_reusable_dir_path) {
                        println!("!!!!!!!!!!!!!FOUND A NEW FILE !!!!!!!!!!!!!!!!");
                        println!("Adding the file: {} to the list of files to recompile, since it uses the modified component '{}'", html_file_str, associated_reusable_name);
                        current_modified.insert(html_file_str.clone());

                        // Insert into modified_html_components only if it belongs in components_dir
                        if html_file_str.contains(components_dir) {
                            println!("Also a component! adding to the components list...");
                            modified_html_components.insert(html_file_str.clone());
                        }
                        println!("  ()()()()() updated modified after adding: {:?}", current_modified);
                        break; // Stop checking this file — it's already added
                    }
                    println!();
                }
            }
            
            // TO NOTE:
            // the next stages check whether the list has changed
            // this is because 
            // in the scenario:
            //	- componentB), imports one of the components that was changed (componentA)
            // componentB needs to be re-compiled, because the component it imports has changed, and so now that code is out of date 
            // This then continues down the tree, for anything that imports componentB, and so on.
            // So we keep looping until all of these have been resolved.

            // === Step 7 & 8: If no new files were added, stop looping ===
            // We just check if the size has changed.
            println!(" 7777777777777777777777777777777777777777777777777");
            println!("updated modified count: {}", current_modified.len());
            println!("current modified count: {}", before_count);

            if current_modified.len() == before_count {
                break;
            }

            println!(" 8888888888888888888888888888888888888888888888888");
            println!("8) the updated modified is not the same as the current modified, looping again.");
            // === Step 9: Otherwise, continue the loop with the updated list ===
            // No need to copy variables; the loop just continues with the modified `current_modified` HashSet.
        }
    }

    println!("()()()()()() returning the current modified: {:?}", current_modified);

    // // === Step 10: Output the final list ===
    // // Convert the HashSet back to a newline-separated string.
    // let mut final_list: Vec<String> = current_modified.into_iter().collect();
    // final_list.sort(); // Sort for deterministic output
    // final_list.join("\n")
    current_modified
}
