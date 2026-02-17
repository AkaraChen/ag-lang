/// Resolves a `std:` module path to its embedded AG source code.
///
/// The path should be the part after `std:`, e.g. `"web/fetch"`, `"log"`, `"fs"`.
/// Returns `None` if the module is not found.
pub fn resolve_std_module(path: &str) -> Option<&'static str> {
    match path {
        // Layer A: Web standard extern declarations (zero runtime)
        "web/fetch" => Some(include_str!("../modules/web/fetch.ag")),
        "web/crypto" => Some(include_str!("../modules/web/crypto.ag")),
        "web/encoding" => Some(include_str!("../modules/web/encoding.ag")),
        "web/streams" => Some(include_str!("../modules/web/streams.ag")),
        "web/timers" => Some(include_str!("../modules/web/timers.ag")),
        // Layer B: AG wrappers (runtime via @agentscript/stdlib)
        "log" => Some(include_str!("../modules/log.ag")),
        "encoding" => Some(include_str!("../modules/encoding.ag")),
        "env" => Some(include_str!("../modules/env.ag")),
        "fs" => Some(include_str!("../modules/fs.ag")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_known_modules() {
        assert!(resolve_std_module("web/fetch").is_some());
        assert!(resolve_std_module("log").is_some());
        assert!(resolve_std_module("fs").is_some());
        assert!(resolve_std_module("encoding").is_some());
        assert!(resolve_std_module("env").is_some());
    }

    #[test]
    fn resolve_unknown_module() {
        assert!(resolve_std_module("nonexistent").is_none());
        assert!(resolve_std_module("web/nonexistent").is_none());
    }
}
