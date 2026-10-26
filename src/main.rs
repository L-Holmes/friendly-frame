use std::process::{Command, exit};
use clap::{Arg, Command as ClapCommand};
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use std::collections::HashSet;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::time::SystemTime;
use serde_json::Value;

use website_compiler::components::*;

// Global definitions
// --> main directories / filenames
const OUTPUT_DIRECTORY: &str = "actual-website-do-not-edit";
const SOURCE_DIR: &str = "edit-me";
const SHARED_CODE_FOLDER: &str = "code";
const LAST_COMPILE_TIME_FILE: &str = ".last_compiled";
const COMPILATION_ORDER_FILE: &str = "reusables-compilation-order.txt";

const SHARED_DIR: &str = "edit-me/shared";

const COMPONENTS_DIR: &str = "edit-me/shared/reusables";
const PAGE_TEXT_DIRECTORY: &str = "edit-me/shared/page_text"; // Directory containing en.json, es.json, etc.
const DEFAULT_LANGUAGE: &str = "en"; // The plain page (e.g. index.html) gets this language's text, so it's the same as en-index.html

const BLOG_TEMPLATE_TOP: &str = "edit-me/shared/reusables/template-blog-top/template-blog-top.html";
const BLOG_TEMPLATE_BOTTOM: &str = "edit-me/shared/reusables/template-blog-bottom/template-blog-bottom.html";
const BLOG_OUTPUT_FOLDER: &str  = "actual-website-do-not-edit/blog"; //Anything in this folder or one of its subfolders is a blog!

// --> Placeholders
const ROOT_PLACEHOLDER: &str = "<root>";
const SCSS_IMPORT_START: &str = "@use";
const HTML_COMPONENT_TEMPLATE_PREFIX: &str = "@template:";
const HTML_COMPONENT_NONE_PREFIX: &str = "@none:";
const RE_START: &str = "<r-"; // start of reusable html component placeholders
const RE_END: &str = ">";
const RE_PARAM_S: &str = "{"; // start of reusable html components parameter placeholders
const RE_PARAM_E: &str = "}"; // end of reusable html components parameter placeholders
const DISPL_NAME_PLACEHOLD_START: &str = "["; // start of reusable html components display name parameter placeholders
const DISPL_NAME_PLACEHOLD_END: &str = "]";     // end of reusable html components display name parameter placeholders

// --> Markers
const BLOG_TEMPLATE_TOP_MARKER: &str = "<!--startofblogtemplatetop-->";

// --> Debug flags
// (These would be used as needed in your actual implementation)

fn main() {
    let matches = ClapCommand::new("website-compiler")
        .version("1.0")
        .about("Website compiler with reusable components")
        .arg(
            Arg::new("fresh")
                .short('f')
                .long("fresh")
                .action(clap::ArgAction::SetTrue)
                .help("Fresh compile; delete the existing output directory contents and compile everything from scratch")
        )
        .arg(
            Arg::new("github-pages")
                .long("github-pages")
                .action(clap::ArgAction::SetTrue)
                .help("Compile with GitHub Pages option")
        )
        .get_matches();

    // Get the options passed
    let github_pages = matches.get_flag("github-pages");
    let fresh_run = matches.get_flag("fresh");

    println!("----------------------------------------------------------------------------------------------------------");
    println!(" --ENSURING REQUIRED THINGS EXIST --");
    println!("----------------------------------------------------------------------------------------------------------");

    // Check for required dependencies
    check_command_exists("sass", "Sass is not installed. Please install Sass to compile .scss files. (https://sass-lang.com/install/)");

    // Your main compilation logic would go here
    // For now, just showing the structure with the flags
    println!("GitHub Pages mode: {}", github_pages);
    println!("Fresh run mode: {}", fresh_run);
    
    // MAIN CODE
    if let Err(e) = run_build_process(fresh_run, github_pages) {
        eprintln!("\n❌ Build failed: {}", e);
        // Add context for chained errors
        let mut cause = e.source();
        while let Some(source) = cause {
            eprintln!("    Caused by: {}", source);
            cause = source.source();
        }
        exit(1);
    }
}

// ========================================================================================================
// -- PREPPING LOCAL FUNCTIONS --
// ========================================================================================================

/// The main entry point for the build process. Orchestrates finding, preparing,
/// and compiling all necessary files.
fn run_build_process(fresh_run: bool, github_pages: bool) -> Result<()> {
    println!("----------------------------------------------------------------------------------------------------------");
    println!(" -- RUNNING LOGIC --");
    println!("----------------------------------------------------------------------------------------------------------");

    let last_compile_time:u64 = if fresh_run {
        println!("RUNNING WITH FRESH RUN!!!!!!!!!!!!!!!!!!!!!!!!!!");
        // clear existing output dir
        if Path::new(OUTPUT_DIRECTORY).exists() {
            fs::remove_dir_all(OUTPUT_DIRECTORY)?;
        }
        fs::create_dir_all(OUTPUT_DIRECTORY)?;
        // reset last compile time
        0
    } else {
        read_last_compile_time(LAST_COMPILE_TIME_FILE)
    };

    let all_files = files_in_source_dir(SOURCE_DIR)?;
    let newly_modified_files = only_modify_time_newer_than_last_compile_time(&all_files, last_compile_time)?;
    let newly_modified_files_set: HashSet<String> = newly_modified_files.iter().map(|p| p.to_string_lossy().to_string()).collect();
    let all_files_that_are_to_be_compiled: HashSet<String> = get_all_files_that_need_recompiling(SOURCE_DIR, RE_START, RE_END, &newly_modified_files_set, COMPONENTS_DIR);


    // TRANSLATION HTML FILES: Load all translation files from the translations directory
    println!("Loading translation files...");
    let translations_files: TranslationsFile = load_translation_files_from_directory(PAGE_TEXT_DIRECTORY).map_err(|e| anyhow::anyhow!("{}", e))?;
    
    println!("Successfully loaded {} language translation files", translations_files.len());
    for available_language_code in translations_files.keys() {
        println!("  - Available language: {}", available_language_code);
    }

    // (0); Load the prioritized components list
    let components_list = load_components_list()?;
    
    // (2) split the newly_modified_files into newly_modified_components and newly_modified_pages_etc
    println!("\n()()()()()()()()()()()() splitting components ()()()()()()()()()()()()");

    // (3) Compiles the components, in the order they should be compiled in.
    println!("\n<><><><><><><><><><><><> COMPILING THE *PRIORITIZED* REUSABLE COMPONENTS IN ORDER <><><><><><><><><><><><>");
    // ---------------------
    let mut compiled_components = HashSet::new();
    let mut all_ts_files = Vec::new();
    let mut all_scss_files = Vec::new();

    // a. Iterate through components in order of priority
    for component_name in &components_list {
        // e.g. "reusables/top-bar"
        let component_dir = Path::new(SHARED_DIR).join(component_name);

        if !component_dir.is_dir() {
            eprintln!("Warning: component directory not found: {}", component_dir.display());
            continue;
        }

        // Iterate over all files in the component’s folder
        for entry in fs::read_dir(&component_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip subdirectories if you only want direct files in the component folder
            if path.is_dir() {
                continue;
            }

            let component_path_str: String = path.to_string_lossy().to_string();

            println!("....................checking whether the following path is in the list of new components that need compiling: {}", &component_path_str);
            if !all_files_that_are_to_be_compiled.contains(&component_path_str) {
                continue;
            }

            println!("\t~~~~~~~");
            println!("\tCompiling (prioritized): {}", &component_path_str);
            println!("\t~~~~~~~");

            let mut single_file_set: HashSet<String> = HashSet::new();
            single_file_set.insert(component_path_str.clone());
            let (ts_files, scss_files) = compile_all(&single_file_set, &translations_files)?;

            all_ts_files.extend(ts_files);
            all_scss_files.extend(scss_files);
            compiled_components.insert(component_path_str);
        }
    }

    // b. Compile any remaining components that weren't in the priority list (i.e. any of the standard pages // things not in the shared directory)
    let new_everything_else_that_needs_compiling : HashSet<String> = get_non_prioritised_files_list(&all_files_that_are_to_be_compiled, &compiled_components);

    println!("<><><><><><><><><><><><> DONE <><><><><><><><><><><><>");
    println!("\n<><><><><><><><><><><><> COMPILING THE REGULAR COMPONENTS IN ORDER <><><><><><><><><><><><>");

    // (4) run compile All on the newly_modified_pages
    let (ts_files, scss_files) = compile_all(&new_everything_else_that_needs_compiling,&translations_files)?;
    all_ts_files.extend(ts_files);
    all_scss_files.extend(scss_files);

    // ----------------------------------------------------------------------------------------
    // --- Final Compilation Step ---
    // ----------------------------------------------------------------------------------------

    // -- compile scss ---
    let mut successful_scss_compilations = 0;
    for css_file in &all_scss_files {
        let scss_source = css_file.with_extension("scss");
        if compile_scss_file(scss_source.to_str().unwrap(), css_file.to_str().unwrap()).is_ok() {
            successful_scss_compilations += 1;
        }
    }
    println!("{} of {} SCSS file(s) compiled to CSS.", successful_scss_compilations, all_scss_files.len());

    // --- compile typescript ---
    let mut successful_ts_compilations = 0;
    for js_file in &all_ts_files {
        let ts_source = js_file.with_extension("ts");
        if compile_typescript_file(ts_source.to_str().unwrap(), js_file.to_str().unwrap()).is_ok() {
            successful_ts_compilations += 1;
        }
    }
    println!("{} of {} TypeScript file(s) compiled to JavaScript.", successful_ts_compilations, all_ts_files.len());
    
    // Clean up .map files created by sass/tsc
    for entry in WalkDir::new(OUTPUT_DIRECTORY).into_iter().filter_map(Result::ok) {
        if entry.path().extension().map_or(false, |e| e == "map") {
            let _ = fs::remove_file(entry.path());
        }
    }

    println!("✅ finished compiling modified files");

    // --- APPLYING GITHUB PAGES COMPILATION OPTIONS ---
    if github_pages {
        println!("----------------------------------------------------------------------------------------------------------");
        println!(" -- APPLYING GITHUB PAGES COMPILATION OPTIONS --");
        println!("----------------------------------------------------------------------------------------------------------");
        let html_files: Vec<_> = WalkDir::new(OUTPUT_DIRECTORY)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "html"))
            .collect();

        for entry in html_files {
            let file_path = entry.path();
            println!("File: {}", file_path.display());
            let mut content = fs::read_to_string(file_path)?;
            // This regex removes `.html` from href attributes, which is safer than a global replace.
            let link_regex = Regex::new(r#"(<a[^>]*\s+href\s*=\s*["'])([^"']*)(\.html)(["'][^>]*>)"#)?;
            content = link_regex.replace_all(&content, r#"$1$2$4"#).to_string();
            fs::write(file_path, content)?;
        }
    }
    
    // Update the last compile time in the hidden file
    update_last_compile_time(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_millis() as u64, LAST_COMPILE_TIME_FILE)?;

    println!("done.");
    Ok(())
}


// TODO! the below function should perhaps be moved into the other file.
/// Given all modified files and the set of already compiled components,
/// returns a newline-separated list of the "non-prioritised" files (everything else).
///
/// # Arguments
/// * `all_modified_files` - A newline-separated string of all files that changed.
/// * `compiled_components` - A set of paths that were already compiled (prioritised).
///
/// # Returns
/// * A single string with one file per line, containing everything that wasn’t already compiled.
pub fn get_non_prioritised_files_list( all_modified_files: &HashSet<String>, compiled_components: &HashSet<String>,) -> HashSet<String> {
    println!("-> filtering non-prioritised files");
    let mut non_prioritised = HashSet::new();
    
    for source_path in all_modified_files {
        if compiled_components.contains(source_path) {
        } else {
            non_prioritised.insert(source_path.clone());
        }
    }
    non_prioritised
}


/// Pre-processes a list of source files: copies them to the output directory,
/// injects/replaces placeholders and components, and returns lists of files
/// that require final compilation (TS and SCSS).
/// This is the Rust version of the `compileAll` shell function.
pub fn compile_all(source_files: &HashSet<String>, translations_files: &TranslationsFile) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut modified_ts_files_list = Vec::new();
    let mut modified_scss_files_list = Vec::new();
    let pages_regex : Regex = Regex::new(r"pages/[^/]+/")?;

    // Iterate through each file path in the HashSet
    for source_path_str in source_files {
        // e.g. source_path=edit-me/pages/index/index.ts  //  edit-me/shared/reusables/top-bar/top-bar.scss
        // println!(" \n~~~\n Compiling: {}\n~~~", source_path_str);

        // Skip empty lines
        if source_path_str.trim().is_empty() {
            continue;
        }
        let source_path: &Path = Path::new(source_path_str);

        // Step 0)a): Generate destination path
        // In Rust, we build this path step-by-step, starting with the path relative to the source dir.
        let path_relative_to_src: &Path = source_path.strip_prefix(SOURCE_DIR).with_context(|| format!("Error stripping prefix '{}' from '{}'", SOURCE_DIR, source_path.display()))?;

        // 1. Replace pages/.*/. with / (move two levels back if in pages)
        // This solves the E0716 error by ensuring `relative_path_cow` lives long enough.
        let relative_path_cow = path_relative_to_src.to_string_lossy();
        let dest_path_str = pages_regex.replace(&relative_path_cow, ""); // e.g. index.ts // shared/reusables/top-bar/top-bar.scss

        // 3. Add OUTPUT_DIRECTORY prefix
        let path_relative_to_src_dir : &Path= Path::new(dest_path_str.as_ref());
        let dest_path_with_swapped_ext : PathBuf= {
            let mut p = path_relative_to_src_dir.as_os_str().to_owned();
            if dest_path_str.ends_with(".ts") {
                p.push(".js");
                PathBuf::from(p.to_string_lossy().replace(".ts.js", ".js"))
            } else if dest_path_str.ends_with(".scss") {
                p.push(".css");
                PathBuf::from(p.to_string_lossy().replace(".scss.css", ".css"))
            } else {
                PathBuf::from(path_relative_to_src_dir)
            }
        };
        let dest_path : PathBuf = Path::new(OUTPUT_DIRECTORY).join(&dest_path_with_swapped_ext);
        // println!("dest_path: {}", dest_path.display());


        //FaQs: Why do we pass in desination uncompiled?
        //      --> Rather confusingly, this is the file's final resting place
        //      --> Its just scss and ts haven't been compiled yet.
        //      --> THey get compiled in-place... So we make all necessary changes before that...
        //      --> For html, the desination uncompiled is the same as desination compiled... so use interchangeably.

        // Step 0)b): Get uncompiled version of destination path
        let dest_uncompiled : PathBuf = Path::new(OUTPUT_DIRECTORY).join(path_relative_to_src_dir);

        // Add to the appropriate array, used for compilation later on
        if dest_uncompiled.extension().and_then(|s| s.to_str()) == Some("ts") {
            modified_ts_files_list.push(dest_path.clone());
        } else if dest_uncompiled.extension().and_then(|s| s.to_str()) == Some("scss") {
            modified_scss_files_list.push(dest_path.clone());
        }

        // println!("        -----------------------------");
		// println!("	1)");
        
        // Step 1: Copy across
        // Delete existing compiled file if it exists (copy will overwrite, but this is for faithfulness)
        if dest_path.exists() {
            let _ = fs::remove_file(&dest_path);
        }
        
        // Copy the file
		if let Some(parent) = dest_uncompiled.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent directory for '{}'", dest_uncompiled.display()))?;
        }
        fs::copy(source_path, &dest_uncompiled)
            .with_context(|| format!("Failed to copy '{}' to '{}'", source_path.display(), dest_uncompiled.display()))?;
        
        // println!("Successfully copied file {} to its destination: {}", source_path.display(), dest_uncompiled.display());


		// Step 1b) Handle blogs
        // --> Determine if its a blog
        // --> Add the reused start and end html code that we repeat across every page
        // --> (done before the steps below, so the blog templates can use <r-...> components and <root> too)
        // e.g. dest_uncompiled may be actual-website-do-not-edit/index.html 
        // e.g. dest_uncompiled may be actual-website-do-not-edit/blog/start-here.html
        // e.g. dest_uncompiled may be actual-website-do-not-edit/blog/gentle/waves.html
        // etc.
        if dest_uncompiled.extension().and_then(|s| s.to_str()) == Some("html") {
            // Check if the file path starts with our designated blog output folder
            if dest_uncompiled.starts_with(BLOG_OUTPUT_FOLDER) {
                ensure_blog_file_boilerplate(&dest_uncompiled)?;
            }
        }

		// println!("        -----------------------------");
		// println!("	2)");
		// Step 2) Replace scss import placeholders
        if dest_uncompiled.extension().and_then(|s| s.to_str()) == Some("scss") {
			// println!("2222222! Is an scss file!");
			replace_root_placeholder_with_relative_path_new(SCSS_IMPORT_START, ROOT_PLACEHOLDER, &dest_uncompiled)?;
		}

		// println!("        -----------------------------");
		// println!("	3)");
		// Step 3) Add reusable components to javascript files
        if dest_uncompiled.extension().and_then(|s| s.to_str()) == Some("ts") {
			// println!("33333333! Is a ts file!");
			add_reusable_javascript_components(&dest_uncompiled, Path::new(OUTPUT_DIRECTORY), SHARED_CODE_FOLDER, RE_START, RE_END)?;
		}

		// println!("        -----------------------------");
		// println!("	4)");
		// Step 4) Add reusable components to html files
        if dest_uncompiled.extension().and_then(|s| s.to_str()) == Some("html") {
			// println!("4! Is a html file!");
            replace_html_component_placeholders( &dest_uncompiled, Path::new(SOURCE_DIR), RE_START, RE_END, ROOT_PLACEHOLDER, HTML_COMPONENT_TEMPLATE_PREFIX, HTML_COMPONENT_NONE_PREFIX, RE_PARAM_S, RE_PARAM_E,DISPL_NAME_PLACEHOLD_START, DISPL_NAME_PLACEHOLD_END)?;
		}

		// println!("        -----------------------------");
		// println!("	5)");
		// Step 5) Replace <root> placeholders with the relative path to root
		// i.e. change <root>/shared/example.html in the file /pages/index.html to ../example.html
		replace_root_placeholder_with_relative_path_new(ROOT_PLACEHOLDER, ROOT_PLACEHOLDER, &dest_uncompiled)?;
        // println!("        -----------------------------");


        // println!("        -----------------------------");
		// println!("	6)");
        // Step 7) HTML files: generate all variations of that file in different languages
        // TRANSLATION HTML FILES: Process all HTML template files and generate language-specific versions
        // println!("        -----------------------------");
		// println!("	7)");
        if dest_uncompiled.extension().and_then(|s| s.to_str()) == Some("html") {
            process_html_template_file_for_all_languages(&dest_uncompiled, OUTPUT_DIRECTORY, &translations_files).map_err(|e| anyhow!("{}", e))?;
        }
        println!("        _____________________________");

    }

    Ok((modified_ts_files_list, modified_scss_files_list))
}



fn check_command_exists(command: &str, error_message: &str) {
    match Command::new("which").arg(command).output() {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("{}", error_message);
                exit(1);
            }
        }
        Err(_) => {
            // Fallback: try to run the command directly
            match Command::new(command)
                .arg("--version")
                .output()
            {
                Ok(_) => {
                    // Command exists
                }
                Err(_) => {
                    eprintln!("{}", error_message);
                    exit(1);
                }
            }
        }
    }
}


/// Loads and validates the prioritized list of components from the compilation order file.
fn load_components_list() -> Result<Vec<String>> {
    let order_file_path = Path::new(SHARED_DIR).join(COMPILATION_ORDER_FILE);
    println!("><><><><><><><><><><>< loading the ordered components list from {} ><><><><><><><><><><><", order_file_path.display());

    if !order_file_path.is_file() {
        return Err(anyhow!("Error: Compilation order file not found at: {}", order_file_path.display()));
    }

    //// Get valid directory names in components_dir (basename only)
    //let valid_dirs: HashSet<String> = WalkDir::new(SHARED_DIR).min_depth(1).max_depth(1).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    // Get all valid directory paths relative to SHARED_DIR (no max depth)
    let valid_dirs: HashSet<String> = WalkDir::new(SHARED_DIR).min_depth(1).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_dir()).map(|e| e.path().strip_prefix(SHARED_DIR).unwrap().to_string_lossy().into_owned()).collect();

    // Read file, clean lines, and validate
    let file_content = fs::read_to_string(&order_file_path)?;
    let mut components_list = Vec::new();
    for line in file_content.lines() {
        let trimmed_line = line.trim();

        // Skip empty lines and comments
        if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
            continue;
        }

        // Check if it's a valid directory
        if valid_dirs.contains(trimmed_line) {
            components_list.push(trimmed_line.to_string());
        } else {
            eprintln!("Valid component directories are: {:?}", valid_dirs);
            return Err(anyhow!("Error: Invalid component name in order file: \"{}\"", trimmed_line));
        }
    }
    
    println!("Validated components list:\n{:?}", &components_list);
    println!("><><><><><><><><><><>< done ><><><><><><><><><><><");
    Ok(components_list)
}


// ============================================================
// GENERATING TRANSLATION FILES
// ============================================================


// Object to represent the contents of the language files. e.g. en.json's contents
type TranslationsFile = HashMap<String, HashMap<String, HashMap<String, String>>>;

/// Process a single HTML template file for all available languages
/// 
/// @param `html_template_file_path` - Path to the HTML template file to process
/// @param `output_directory_path` - Directory where the localized HTML files will be generated
/// @param `translation_cache` - Map containing translations for different languages
/// 
/// @return `Ok(())` - If all language files are generated successfully; otherwise `Err(Box<dyn std::error::Error>)` - If any error occurs during processing
/// 
/// # Example
/// let html_file = "template.html";
/// let output_dir = "output";
/// let translations = TranslationsFile::new();
/// process_html_template_file_for_all_languages(html_file, output_dir, &translations)?;
/// ```
fn process_html_template_file_for_all_languages<P1: AsRef<Path>, P2: AsRef<Path>>( html_template_file_path: P1, output_directory_path: P2, translation_cache: &TranslationsFile) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nPROCESSING HTML TEMPLATE FILE FOR ALL LANGUAGES");
    let html_file_path_reference: &Path = html_template_file_path.as_ref();
    let html_base_filename: &str = html_file_path_reference.file_stem().and_then(|os_string: &std::ffi::OsStr| os_string.to_str())
        .ok_or_else(|| format!("Invalid HTML filename: {:?}", html_file_path_reference))?;
    println!("Html file basename: {}",html_base_filename);
    
    // Generate a translated file for each available language
    for target_language_code in translation_cache.keys() {
        // let localized_output_filename: String = format!("{}-{}.html", target_language_code, html_base_filename);
        // let localized_output_file_path: PathBuf = output_directory_path.as_ref().join(localized_output_filename);

        let localized_output_filename = format!("{}-{}.html", target_language_code, html_base_filename);
        let localized_output_file_path = html_file_path_reference .parent() .unwrap_or_else(|| Path::new(output_directory_path.as_ref())) .join(localized_output_filename);
        println!("got the localised path...");
        
        
        generate_language_file( html_file_path_reference, &localized_output_file_path, translation_cache, target_language_code)
        .map_err(|error| {
            format!( "Failed to generate language file for {} in {}: {}", target_language_code, html_base_filename, error)
        })?;
    }

    // The plain page (e.g. index.html) becomes the default language's version, instead of being left with the untranslated TEXT= placeholders
    if translation_cache.contains_key(DEFAULT_LANGUAGE) {
        let default_language_file_path = html_file_path_reference.parent().unwrap_or_else(|| Path::new(output_directory_path.as_ref())).join(format!("{}-{}.html", DEFAULT_LANGUAGE, html_base_filename));
        fs::copy(&default_language_file_path, html_file_path_reference)
            .map_err(|error| format!("Failed to copy {:?} to {:?}: {}", default_language_file_path, html_file_path_reference, error))?;
    }
    
    Ok(())
}


/// Load all JSON translation files from a directory containing language files like en.json, es.json
fn load_translation_files_from_directory<P: AsRef<Path>>( translation_directory_path: P) -> Result<TranslationsFile, Box<dyn std::error::Error>> {
    let directory_entries: fs::ReadDir = fs::read_dir(&translation_directory_path)
        .map_err(|error: std::io::Error| {
            format!("Failed to read translation directory {:?}: {}", translation_directory_path.as_ref(),
            error)
        })?;
    
    let mut json_file_paths: Vec<PathBuf> = Vec::new();
    
    for directory_entry in directory_entries {
        let directory_entry: fs::DirEntry = directory_entry?;
        let file_path: PathBuf = directory_entry.path();
        
        if let Some(file_extension) = file_path.extension() {
            if file_extension == "json" {
                json_file_paths.push(file_path);
            }
        }
    }
    
    load_translation_files_from_paths(&json_file_paths)
}

/// Load translations from specific JSON file paths
fn load_translation_files_from_paths<P: AsRef<Path>>( json_file_paths: &[P]) -> Result<TranslationsFile, Box<dyn std::error::Error>> {
    let mut translation_cache_map: TranslationsFile = HashMap::with_capacity(json_file_paths.len());
    
    for json_file_path in json_file_paths {
        let json_file_path_reference: &Path = json_file_path.as_ref();
        
        // Extract language code from filename (e.g., "en.json" -> "en")
        let language_code: &str = json_file_path_reference.file_stem().and_then(|os_string: &std::ffi::OsStr| os_string.to_str()).ok_or_else(|| format!("Invalid JSON file name: {:?}", json_file_path_reference))?;
        
        let json_file_content: String = fs::read_to_string(json_file_path_reference)
            .map_err(|error: std::io::Error| {
                format!("Failed to read translation file {:?}: {}", json_file_path_reference, error)
            })?;
        
        let parsed_json: Value = serde_json::from_str(&json_file_content)
            .map_err(|error: serde_json::Error| {
                format!("Failed to parse JSON in {:?}: {}", json_file_path_reference, error)
            })?;
        
        let mut language_translation_map: HashMap<String, HashMap<String, String>> = HashMap::new();
        
        if let Value::Object(page_objects_map) = parsed_json {
            for (page_name, page_content_value) in page_objects_map {
                if let Value::Object(page_variables_map) = page_content_value {
                    let page_variable_translations: HashMap<String, String> = page_variables_map
                        .into_iter()
                        .filter_map(|(variable_name, variable_value)| {
                            variable_value.as_str().map(|translation_text: &str| {
                                (variable_name, translation_text.to_string())
                            })
                        })
                        .collect();
                    
                    language_translation_map.insert(page_name, page_variable_translations);
                }
            }
        }
        
        translation_cache_map.insert(language_code.to_string(), language_translation_map);
    }
    
    Ok(translation_cache_map)
}

/// Generate a language-specific HTML file by replacing translation placeholders
fn generate_language_file<P1: AsRef<Path>, P2: AsRef<Path>>( html_file_path: P1, output_file_path: P2, parsed_json: &TranslationsFile, target_language_code: &str) -> Result<(), Box<dyn std::error::Error>> {
    let original_html_content: String = fs::read_to_string(&html_file_path)
        .map_err(|error: std::io::Error| {
            format!("Failed to read HTML file {:?}: {}", html_file_path.as_ref(), error)
        })?;
    
    // Create regex for finding translation placeholders with improved pattern
    let translation_placeholder_regex: Regex = Regex::new(r"<div>TEXT=([a-zA-Z0-9_\.]+)</div>")
        .expect("Failed to compile translation placeholder regex");
    
    let processed_html_content: String = translation_placeholder_regex.replace_all(&original_html_content, |regex_captures: &regex::Captures| {
            let full_translation_key: &str = &regex_captures[1]; // e.g., "pages.index.title"
            
            if let Some((page_name, variable_name)) = full_translation_key.split_once('.') {
                // Try to get translation for this language
                if let Some(translated_text) = get_translation_for_language_page_variable( parsed_json, target_language_code, page_name, variable_name) {
                    format!("<div>{}</div>", translated_text)
                } else {
                    // Fallback to original key if translation not found
                    format!("<div>{}</div>", full_translation_key)
                }
            } else {
                // Invalid key format, keep original placeholder
                format!("<div>TEXT={}</div>", full_translation_key)
            }
        }).to_string();
    
    // Write the processed content to the output file
    fs::write(&output_file_path, processed_html_content)
        .map_err(|error: std::io::Error| {
            format!("Failed to write output file {:?}: {}", output_file_path.as_ref(), error)
        })?;
    
    println!("Generated language file: {:?}", output_file_path.as_ref());
    Ok(())
}


/// Inline lookup function for maximum translation retrieval speed
#[inline]
fn get_translation_for_language_page_variable<'a>( translation_cache: &'a TranslationsFile, language_code: &str, page_name: &str, variable_name: &str) -> Option<&'a str> {
    translation_cache.get(language_code)
        .and_then(|language_map: &HashMap<String, HashMap<String, String>>| {
            language_map.get(page_name)
        })
        .and_then(|page_variables_map: &HashMap<String, String>| {
            page_variables_map.get(variable_name)
        })
        .map(|translation_string: &String| translation_string.as_str())
}



fn ensure_blog_file_boilerplate(path: &std::path::Path) -> anyhow::Result<()> {
    // 1. Read the blog file - adding context so we know if THIS is the file missing
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read blog file at: {:?}", path))?;

    // 2. CHECK: Does it have the marker?
    let already_has_boilerplate = content
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().contains(BLOG_TEMPLATE_TOP_MARKER))
        .unwrap_or(false);

    if !already_has_boilerplate {
        println!("--> Applying boilerplate: {:?}", path);
        println!("Running from: {:?}", std::env::current_dir()?);

        // 3. Read templates with specific error messages
        // These are often the culprits for "os error 2" if the CWD is wrong!
        let top = std::fs::read_to_string(BLOG_TEMPLATE_TOP)
            .with_context(|| format!("MISSING TOP TEMPLATE: Checked path '{}'. Is your terminal in the right folder?", BLOG_TEMPLATE_TOP))?;
            
        let bottom = std::fs::read_to_string(BLOG_TEMPLATE_BOTTOM)
            .with_context(|| format!("MISSING BOTTOM TEMPLATE: Checked path '{}'.", BLOG_TEMPLATE_BOTTOM))?;

        let formatted_output = format!("{}\n{}\n{}", top, content, bottom);

        // 4. Write back
        std::fs::write(path, formatted_output)
            .with_context(|| format!("Failed to write updated content back to: {:?}", path))?;
    }

    Ok(())
}
