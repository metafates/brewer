use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct State<F, C> {
    pub formulae: F,
    pub casks: C,
}

pub mod formula {
    use std::collections::HashSet;

    use serde::{Deserialize, Serialize};

    use crate::models::keg;

    pub type Executables = keg::Store<HashSet<String>>;
    pub type State = keg::State<Formula, installed::Formula>;
    pub type Store = keg::Store<Formula>;

    #[derive(Deserialize, Serialize, Clone)]
    pub struct Formula {
        pub base: base::Formula,
        pub executables: HashSet<String>,
        pub analytics: Option<analytics::Formula>,
    }

    impl AsRef<str> for Formula {
        fn as_ref(&self) -> &str {
            &self.base.name
        }
    }

    pub mod base {
        use std::collections::HashSet;

        use serde::{Deserialize, Serialize};

        use crate::models::formula::installed;
        use crate::models::keg;

        pub type State = keg::State<Formula, installed::Formula>;
        pub type Store = keg::Store<Formula>;

        #[derive(Serialize, Deserialize, Clone)]
        pub struct Formula {
            pub name: String,
            pub tap: String,
            pub desc: String,
            pub homepage: String,

            #[serde(default)]
            pub aliases: HashSet<String>,

            pub versions: Versions,
        }

        #[derive(Serialize, Deserialize, Clone)]
        pub struct Versions {
            pub stable: String,
            pub head: Option<String>,
        }
    }

    pub mod installed {
        use serde::{Deserialize, Serialize};

        use crate::models::formula::receipt;
        use crate::models::keg;

        pub type Store = keg::Store<Formula>;

        #[derive(Serialize, Deserialize, Clone)]
        pub struct Formula {
            pub upstream: super::Formula,
            pub receipt: receipt::Receipt,
        }
    }

    pub mod analytics {
        use std::fmt::Display;
        use std::str::FromStr;

        use serde::{Deserialize, Serialize};

        use crate::models::keg;

        pub type Store = keg::Store<Formula>;

        #[derive(Serialize, Deserialize, Clone)]
        pub struct Formula {
            pub number: i64,
            pub formula: String,
        }
    }

    pub mod receipt {
        use serde::{Deserialize, Serialize};

        use crate::models::keg;

        pub type Store = keg::Store<Receipt>;

        #[derive(Serialize, Deserialize, Clone)]
        pub struct Receipt {
            pub source: Source,
            pub installed_as_dependency: bool,
            pub installed_on_request: bool,
        }

        #[derive(Serialize, Deserialize, Clone)]
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

        #[derive(Serialize, Deserialize, Clone)]
        #[serde(rename_all = "camelCase")]
        pub enum Spec {
            Stable,
            Head,
        }


        #[derive(Serialize, Deserialize, Clone)]
        pub struct Versions {
            pub stable: String,
            pub head: Option<String>,
        }
    }
}

pub mod cask {
    use serde::{Deserialize, Serialize};

    use crate::models::keg;

    pub type State = keg::State<Cask, installed::Cask>;
    pub type Store = keg::Store<Cask>;

    #[derive(Default, Serialize, Deserialize, Clone)]
    pub struct Cask {
        pub base: base::Cask,
    }

    impl AsRef<str> for Cask {
        fn as_ref(&self) -> &str {
            &self.base.token
        }
    }

    pub mod base {
        use std::collections::HashSet;

        use serde::{Deserialize, Serialize};

        use crate::models::cask::installed;
        use crate::models::keg;

        #[derive(Default, Serialize, Deserialize, Clone)]
        pub struct Cask {
            pub token: String,
            pub tap: String,
            pub desc: Option<String>,
            pub version: String,

            pub homepage: String,

            #[serde(default)]
            pub names: HashSet<String>,
        }

        pub type State = keg::State<Cask, installed::Cask>;
        pub type Store = keg::Store<Cask>;
    }

    pub mod installed {
        use std::collections::HashSet;

        use serde::{Deserialize, Serialize};

        use crate::models::keg;

        pub type Store = keg::Store<Cask>;
        pub type VersionsStore = keg::Store<HashSet<String>>;

        #[derive(Serialize, Deserialize, Clone)]
        pub struct Cask {
            pub upstream: super::Cask,
            pub versions: HashSet<String>,
        }
    }
}

pub mod keg {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct State<A, I> {
        pub all: Store<A>,
        pub installed: Store<I>,
    }

    pub type Store<T> = HashMap<String, T>;
}


