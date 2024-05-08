pub struct State<F, C> {
    pub formulae: F,
    pub casks: C,
}

pub mod formula {
    use std::collections::HashSet;

    use serde::Deserialize;

    use crate::models::keg;

    #[derive(Default, Deserialize, Clone)]
    pub struct Formula {
        pub name: String,
        pub tap: String,
        pub desc: String,

        #[serde(default)]
        pub aliases: HashSet<String>,
    }

    pub type State = keg::State<Formula, installed::Formula>;
    pub type Store = keg::Store<Formula>;

    pub mod installed {
        use crate::models::formula::receipt;
        use crate::models::keg;

        pub type Store = keg::Store<Formula>;

        pub struct Formula {
            pub upstream: super::Formula,
            pub receipt: receipt::Receipt,
        }
    }

    pub mod receipt {
        use serde::Deserialize;

        use crate::models::keg;

        pub type Store = keg::Store<Receipt>;

        #[derive(Deserialize)]
        pub struct Receipt {
            pub source: Source,
            pub installed_as_dependency: bool,
            pub installed_on_request: bool,
        }

        #[derive(Deserialize)]
        pub struct Source {
            pub spec: Spec,
            pub versions: Versions,
        }

        impl Source {
            pub fn version(&self) -> String {
                match self.spec {
                    Spec::Stable => self.versions.stable.clone(),
                    Spec::Head => self.versions.head.clone().unwrap_or("HEAD".into()),
                }
            }
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub enum Spec {
            Stable,
            Head,
        }


        #[derive(Deserialize)]
        pub struct Versions {
            pub stable: String,
            pub head: Option<String>,
        }
    }
}

pub mod cask {
    use std::collections::HashSet;

    use serde::Deserialize;

    use crate::models::keg;

    #[derive(Default, Deserialize, Clone)]
    pub struct Cask {
        pub token: String,
        pub tap: String,

        #[serde(default)]
        pub names: HashSet<String>,
    }

    pub type State = keg::State<Cask, installed::Cask>;
    pub type Store = keg::Store<Cask>;

    pub mod installed {
        use std::collections::HashSet;

        use crate::models::keg;

        pub type Store = keg::Store<Cask>;
        pub type VersionsStore = keg::Store<HashSet<String>>;

        pub struct Cask {
            pub cask: super::Cask,
            pub versions: HashSet<String>,
        }
    }
}

pub mod keg {
    use std::collections::HashMap;

    pub struct State<A, I> {
        pub all: Store<A>,
        pub installed: Store<I>,
    }

    pub type Store<T> = HashMap<String, T>;
}


