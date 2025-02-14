use heck::ToSnakeCase;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_dir = out_dir.join("libpg_query");
    let src_dir = PathBuf::from("./lib/libpg_query").canonicalize().unwrap();
    println!(
        "cargo:rerun-if-changed={}",
        src_dir.join("pg_query.h").display()
    );

    // Copy the files over
    eprintln!("Copying {} -> {}", src_dir.display(), build_dir.display());
    let changed = copy_dir(&src_dir, &build_dir).expect("Copy failed");

    // Generate the AST first
    generate_ast(&build_dir, &out_dir).expect("AST generation");

    // Now compile the C library.
    // Only recompile if something changed.
    if changed {
        let mut make = Command::new("make");
        make.env_remove("PROFILE").arg("-C").arg(&build_dir);

        // If we're in debug mode, add the DEBUG flag.
        if env::var("PROFILE").unwrap() == "debug" {
            make.arg("DEBUG=1");
        }

        // **Modification for Windows cross-compilation**
        // If targeting Windows via the GNU toolchain, set the CC variable so that
        // the Makefile uses the MinGW cross-compiler.
        if target.contains("windows-gnu") {
            eprintln!("Target {} detected; using x86_64-w64-mingw32-gcc", target);
            make.env("CC", "x86_64-w64-mingw32-gcc");
            // If you are compiling for 32-bit Windows, you could use:
            // make.env("CC", "i686-w64-mingw32-gcc");
        }

        let status = make
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .unwrap();
        assert!(status.success());
    }

    // Also generate bindings
    let bindings = bindgen::Builder::default()
        .header(build_dir.join("pg_query.h").to_str().unwrap())
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=pg_query");
}

// ... rest of your functions remain the same ...
