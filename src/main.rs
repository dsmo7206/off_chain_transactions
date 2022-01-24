mod io;
mod state;
mod types;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let input_filename = std::env::args()
        .nth(1)
        .ok_or("Input filename not specified")?;

    let mut state = state::State::default();

    for result in io::CsvFileReader::new(&input_filename)? {
        state.process(result?)?;
    }

    // Dump state to stdout
    state.write(std::io::stdout())?;

    Ok(())
}
