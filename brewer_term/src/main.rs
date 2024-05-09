use std::path::PathBuf;

fn run() -> anyhow::Result<()> {
    let store = brewer_engine::store::Store::open(PathBuf::from("brewer.db").as_path())?;

    let mut engine = brewer_engine::EngineBuilder::default()
        .store(store)
        .build()?;

    let state = engine.cache_or_latest()?;

    for (_, f) in state.formulae.all.iter() {
        if state.formulae.installed.contains_key(&f.name) {
            println!("{} {}", f.name, f.versions.stable);
            println!("{}", f.desc);
        }

        // if f.receipt.installed_on_request {
        //     println!("{} {}", f.upstream.name, f.receipt.source.version());
        //     println!("{}", f.upstream.desc);
        //     println!();
        // }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e)
    }
}
