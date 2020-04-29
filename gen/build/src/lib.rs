//! The CXX code generator for constructing and compiling C++ code.
//!
//! This is intended to be used from Cargo build scripts to execute CXX's
//! C++ code generator, set up any additional compiler flags depending on
//! the use case, and make the C++ compiler invocation.
//!
//! <br>
//!
//! # Example
//!
//! Example of a canonical Cargo build script that builds a CXX bridge:
//!
//! ```no_run
//! // build.rs
//!
//! fn main() {
//!     cxx_build::bridge("src/main.rs")
//!         .file("../demo-cxx/demo.cc")
//!         .flag("-std=c++11")
//!         .compile("cxxbridge-demo");
//!
//!     println!("cargo:rerun-if-changed=src/main.rs");
//!     println!("cargo:rerun-if-changed=../demo-cxx/demo.h");
//!     println!("cargo:rerun-if-changed=../demo-cxx/demo.cc");
//! }
//! ```
//!
//! A runnable working setup with this build script is shown in the
//! *demo-rs* and *demo-cxx* directories of [https://github.com/dtolnay/cxx].
//!
//! [https://github.com/dtolnay/cxx]: https://github.com/dtolnay/cxx
//!
//! <br>
//!
//! # Alternatives
//!
//! For use in non-Cargo builds like Bazel or Buck, CXX provides an
//! alternate way of invoking the C++ code generator as a standalone command
//! line tool. The tool is packaged as the `cxxbridge-cmd` crate.
//!
//! ```bash
//! $ cargo install cxxbridge-cmd  # or build it from the repo
//!
//! $ cxxbridge src/main.rs --header > path/to/mybridge.h
//! $ cxxbridge src/main.rs > path/to/mybridge.cc
//! ```

mod error;
mod gen;
mod paths;
mod syntax;

use crate::error::Result;
use crate::gen::Opt;
use anyhow::anyhow;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

/// This returns a [`cc::Build`] on which you should continue to set up any
/// additional source files or compiler flags, and lastly call its [`compile`]
/// method to execute the C++ build.
///
/// [`compile`]: https://docs.rs/cc/1.0.49/cc/struct.Build.html#method.compile
#[must_use]
pub fn bridge(rust_source_file: impl AsRef<Path>) -> cc::Build {
    match try_generate_bridge(rust_source_file.as_ref()) {
        Ok(build) => build,
        Err(err) => {
            let _ = writeln!(io::stderr(), "\n\ncxxbridge error: {:?}\n\n", anyhow!(err));
            process::exit(1);
        }
    }
}

fn try_generate_bridge(rust_source_file: &Path) -> Result<cc::Build> {
    let header = gen::do_generate_header(rust_source_file, Opt::default());
    let header_path = paths::out_with_extension(rust_source_file, ".h")?;
    fs::create_dir_all(header_path.parent().unwrap())?;
    fs::write(&header_path, header)?;
    paths::symlink_header(&header_path, rust_source_file);

    let bridge = gen::do_generate_bridge(rust_source_file, Opt::default());
    let bridge_path = paths::out_with_extension(rust_source_file, ".cc")?;
    fs::write(&bridge_path, bridge)?;
    let mut build = paths::cc_build();
    build.file(&bridge_path);

    let ref cxx_h = paths::include_dir()?.join("rust").join("cxx.h");
    let _ = fs::create_dir_all(cxx_h.parent().unwrap());
    let _ = fs::remove_file(cxx_h);
    let _ = fs::write(cxx_h, gen::include::HEADER);

    Ok(build)
}
