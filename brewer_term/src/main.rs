use std::path::PathBuf;

fn run() -> anyhow::Result<()> {
    let store = brewer_engine::store::Store::open(PathBuf::from("brewer.db").as_path())?;

    let mut engine = brewer_engine::EngineBuilder::default()
        .store(store)
        .build()?;

    let state = engine.cache_or_latest()?;

    for (_, f) in state.formulae.installed.iter() {
        println!("{} {}", f.upstream.base.name, f.receipt.source.version());
        println!("{}", f.upstream.base.desc);

        for e in f.upstream.executables.iter() {
            println!("Provides {}", e);
        }

        println!();
        println!();
        println!();

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
