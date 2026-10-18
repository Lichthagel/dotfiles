fn main() {
    let mut stdout = std::io::stdout();
    if let Err(error) = dotfiles::run(std::env::args_os(), &mut stdout) {
        eprintln!("dotfiles: {error}");
        std::process::exit(error.exit_category().status_code());
    }
}
