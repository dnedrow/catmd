use std::io;
use std::process::ExitCode;

use clap::Parser;

use catmd::cli::Args;
use catmd::env::{Environment, prepare_terminal, resolve};
use catmd::input::{Source, render_all};
use catmd::pager;
use catmd::theme::file as theme_file;

const EXIT_FAILURE: u8 = 1;
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    let args = Args::parse();
    let mut env = Environment::capture();
    if catmd::image::detect::should_probe(&args, &env) {
        env.probe = catmd::image::probe::probe_terminal();
    }
    let mut settings = resolve(&args, &env);
    if let Some(source) = theme_file::select(args.theme.as_deref(), &env) {
        match theme_file::load(&source) {
            Ok(theme) => settings.theme = theme,
            Err(err) => {
                eprintln!("catmd: theme {err}");
                return ExitCode::from(EXIT_USAGE);
            }
        }
    }
    prepare_terminal(&settings);

    let sources = Source::from_args(&args.files);
    let mut failed = false;
    let (output, _) = render_all(&sources, &settings, &mut |source, err| {
        failed = true;
        eprintln!("catmd: {source}: {err}");
    });

    let exit = if failed {
        ExitCode::from(EXIT_FAILURE)
    } else {
        ExitCode::SUCCESS
    };
    match pager::deliver(&settings, &output) {
        Ok(()) => exit,
        // The reader went away (e.g. `catmd doc.md | head`); like `cat`, stop quietly.
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => exit,
        Err(err) => {
            eprintln!("catmd: {err}");
            ExitCode::from(EXIT_FAILURE)
        }
    }
}
