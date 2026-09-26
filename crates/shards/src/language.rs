#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Sh,
    Julia,
}

impl Language {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "rust" => Some(Self::Rust),
            "sh" => Some(Self::Sh),
            "julia" => Some(Self::Julia),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Sh => "sh",
            Self::Julia => "julia",
        }
    }
}
