extern crate getopts;
extern crate miri;
extern crate rustc;
extern crate rustc_driver;
extern crate test;

use rustc_driver::{driver, Compilation};
use rustc::hir::def_id::LOCAL_CRATE;
use std::cell::RefCell;
use std::rc::Rc;

use miri::{MiriConfig, eval_main};

use crate::test::Bencher;

pub struct MiriCompilerCalls<'a>(Rc<RefCell<&'a mut Bencher>>);

fn find_sysroot() -> String {
    // Taken from https://github.com/Manishearth/rust-clippy/pull/911.
    let home = option_env!("RUSTUP_HOME").or(option_env!("MULTIRUST_HOME"));
    let toolchain = option_env!("RUSTUP_TOOLCHAIN").or(option_env!("MULTIRUST_TOOLCHAIN"));
    match (home, toolchain) {
        (Some(home), Some(toolchain)) => format!("{}/toolchains/{}", home, toolchain),
        _ => {
            option_env!("RUST_SYSROOT")
                .expect(
                    "need to specify RUST_SYSROOT env var or use rustup or multirust",
                )
                .to_owned()
        }
    }
}

pub fn run(filename: &str, bencher: &mut Bencher) {
    let args = &[
        "miri".to_string(),
        format!("benches/helpers/{}.rs", filename),
        "--sysroot".to_string(),
        find_sysroot(),
    ];
    let bencher = RefCell::new(bencher);

    let mut control = driver::CompileController::basic();

    control.after_analysis.stop = Compilation::Stop;
    control.after_analysis.callback = Box::new(move |state| {
        state.session.abort_if_errors();

        let tcx = state.tcx.unwrap();
        let (entry_def_id, _) = tcx.entry_fn(LOCAL_CRATE).expect(
            "no main or start function found",
        );

        bencher.borrow_mut().iter(|| {
            let config = MiriConfig { validate: true, args: vec![] };
            eval_main(tcx, entry_def_id, config);
        });

        state.session.abort_if_errors();
    });

    rustc_driver::run_compiler(args, Box::new(control), None, None);
}
