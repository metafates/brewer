fn run() -> anyhow::Result<()> {
    let brew = brewer_core::Brew::default();

    let state = brew.state()?;

    for (_, f) in state.formulae.installed.iter() {
        if f.receipt.installed_on_request {
            println!("{} {}", f.upstream.name, f.receipt.source.version());
            println!("{}", f.upstream.desc);
            println!();
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e)
    }
}
