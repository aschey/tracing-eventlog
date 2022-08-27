use regex::Regex;
use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    env,
    fs::{self, File},
    io::{copy, prelude::*, BufRead, BufReader, LineWriter},
    path::Path,
    process::Command,
    str,
};

// This regex grabs all MC-generated #define statements and for each it
// captures 3 groups: name, cast, value. The "cast" group is optional.
// i.e. "#define SOMETHING   ((DWORD)0x1200L)" -> ("SOMETHING", "DWORD", 0x1200)
const REGEX: &str = r"^#define (\S+)\s+\(?(\([[:alpha:]]+\))?\s*(0x[[:xdigit:]]+)";

const INPUT_FILE: &str = "res/eventmsgs.mc";
const TMPL_FILE: &str = "res/eventmsgs.mc.tmpl";
const GENERATED_FILE: &str = "res/eventmsgs.rs";
const HEADER_FILE: &str = "res/eventmsgs.h";
const LIB_FILE: &str = "res/eventmsgs.lib";

const MC_ARGS: &[&str] = &["-U", "-h", "res", "-r", "res", INPUT_FILE];

#[cfg(not(windows))]
const RC_ARGS: &[&str] = &["-v", "-i", "res/eventmsgs.rc", "-o", "res/eventmsgs.lib"];
#[cfg(windows)]
const RC_ARGS: &[&str] = &["/v", "/fo", "res/eventmsgs.lib", "res/eventmsgs.rc"];

#[cfg(not(windows))]
const MC_BIN: &str = "windmc";

#[cfg(not(windows))]
const RC_BIN: &str = "windres";
#[cfg(windows)]
const RC_BIN: &str = "rc.exe";

const FUNC_TEXT: &str = "
#[allow(unused_variables)]
pub fn get_category(category: String) -> u16 {
{TEXT}
}
";

const MATCH_TEXT: &str = "
    match category.trim().to_lowercase().as_ref() {
        {TEXT}
        _ => 0,
    }
";

#[cfg(not(windows))]
fn prefix_command(cmd: &str) -> Cow<str> {
    let target = env::var("TARGET").unwrap();
    let arch: &str = target.split("-").collect::<Vec<&str>>()[0];
    format!("{}-w64-mingw32-{}", arch, cmd).into()
}

#[cfg(windows)]
const MC_BIN: &str = "mc.exe";
#[cfg(windows)]
fn prefix_command(cmd: &str) -> Cow<str> {
    cmd.into()
}

fn run_tool(cmd: &str, args: &[&str]) -> Result<(), ()> {
    let program = prefix_command(cmd);
    let mut command = Command::new(program.as_ref());
    match command.args(args).output() {
        Ok(out) => {
            println!("{:?}", str::from_utf8(&out.stderr).unwrap());
            println!("{:?}", str::from_utf8(&out.stdout).unwrap());
            Ok(())
        }
        Err(err) => {
            println!("ERROR: Failed to run command: {}, error: {}", program, err);
            Err(())
        }
    }
}

fn gen_rust(origin_hash: &str, category_list: Vec<&str>) {
    let re = Regex::new(REGEX).unwrap();

    let file_out = File::create(GENERATED_FILE).unwrap();
    let mut writer = LineWriter::new(file_out);

    writer
        .write_all(
            format!(
                "// Auto-generated from origin with SHA256 {}.\n",
                origin_hash
            )
            .as_bytes(),
        )
        .unwrap();
    writer
        .write_all(
            format!(
                "pub(crate) const CATEGORY_COUNT: u32 = {};\n\n",
                category_list.len(),
            )
            .as_bytes(),
        )
        .unwrap();

    let file_in = File::open("res/eventmsgs.h").unwrap();
    for line_res in BufReader::new(file_in).lines() {
        let line = line_res.unwrap();
        if let Some(x) = re.captures(&line) {
            let datatype = match x.get(2).map(|v| v.as_str()) {
                Some("(WORD)") => "u16",
                Some("(DWORD)") => "u32",
                _ => "u32",
            };
            writer
                .write_all(format!("pub const {}: {datatype} = {};\n", &x[1], &x[3]).as_bytes())
                .unwrap();
        }
    }

    let func_body = if category_list.is_empty() {
        "0".to_owned()
    } else {
        let match_cases = category_list
            .iter()
            .map(|c| {
                format!(
                    "\"\\\"{}\\\"\" => {},",
                    c.to_lowercase(),
                    get_category_const(c)
                )
            })
            .collect::<Vec<_>>()
            .join("\n        ");
        MATCH_TEXT.replace("{TEXT}", &match_cases)
    };

    let category_func_text = FUNC_TEXT.replace("{TEXT}", &func_body);
    writer.write_all(category_func_text.as_bytes()).unwrap();
}

fn file_hash(f: &str) -> String {
    let mut file = File::open(f).unwrap();
    let mut hasher = Sha256::new();
    let _count = copy(&mut file, &mut hasher).unwrap();
    let formatted = format!("{:x}", hasher.finalize());
    println!("file={}, hash={}", f, formatted);
    formatted
}

fn file_contains(f: &str, needle: &str) -> bool {
    match File::open(f) {
        Err(_) => false,
        Ok(file) => {
            for line in BufReader::new(file).lines() {
                if line.unwrap().contains(needle) {
                    println!("file={} contains {}", f, needle);
                    return true;
                }
            }
            println!("file={} does not contain {}", f, needle);
            false
        }
    }
}

fn delete_if_exists(path: impl AsRef<Path>) {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_file(path).unwrap();
    }
}

fn error_if_not_found(path: impl AsRef<Path>) {
    let path = path.as_ref();
    if !path.exists() {
        panic!("failed to generate {:?}", path);
    }
}

fn get_category_const(category: &str) -> String {
    let category_name = category.replace(' ', "_").to_uppercase();
    format!("{category_name}_CATEGORY")
}

fn gen_category_text(name: &str, id: usize) -> String {
    let name = name.trim();
    let category_name = get_category_const(name);
    format!("MessageId={id:#X}\nSymbolicName={category_name}\nLanguage=English\n{name}\n.\n")
}

fn main() {
    for (key, value) in env::vars() {
        println!("Env[{}]={}", key, value);
    }

    let (categories, category_list) = match option_env!("TRACING_EVENTLOG_CATEGORIES") {
        Some(categories) => {
            let category_list = categories.split(',').map(|c| c.trim()).collect::<Vec<_>>();
            let category_text = category_list
                .iter()
                .enumerate()
                .map(|(i, category)| gen_category_text(category, i + 1))
                .collect::<Vec<_>>()
                .join("\n");

            (
                format!("; // Event categories\n\nMessageIdTypedef=WORD\n\n{category_text}"),
                category_list,
            )
        }
        None => ("".to_owned(), vec![]),
    };
    let file_contents = fs::read_to_string(TMPL_FILE).unwrap();
    let new_contents = file_contents.replace("{CATEGORIES}", &categories);
    fs::write(INPUT_FILE, new_contents).unwrap();
    let origin_hash = file_hash(TMPL_FILE);
    if cfg!(not(windows)) || !file_contains(GENERATED_FILE, &origin_hash) {
        println!(
            "Generating {} from {} with hash {}",
            GENERATED_FILE, INPUT_FILE, origin_hash
        );
        let mc_exe = embed_resource::find_windows_sdk_tool(MC_BIN).unwrap();
        let rc_exe = embed_resource::find_windows_sdk_tool(RC_BIN).unwrap();

        delete_if_exists(HEADER_FILE);
        run_tool(&mc_exe.to_string_lossy().to_string(), MC_ARGS).unwrap();
        error_if_not_found(HEADER_FILE);

        delete_if_exists(LIB_FILE);
        run_tool(&rc_exe.to_string_lossy().to_string(), RC_ARGS).unwrap();
        error_if_not_found(LIB_FILE);
        gen_rust(&origin_hash, category_list);
    }

    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}/res", dir);
    println!("cargo:rustc-link-lib=dylib=eventmsgs");
}
