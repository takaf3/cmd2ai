pub const WEB_SEARCH_KEYWORDS: &[&str] = &[
    "latest",
    "recent",
    "current",
    "today",
    "yesterday",
    "news",
    "update",
    "price",
    "stock",
    "weather",
    "score",
    "result",
    "released",
    "announced",
    "trending",
    "happening",
    "now",
    "breaking",
    "2024",
    "2025",
    "this week",
    "this month",
    "real-time",
    "live",
    "status",
    "outage",
    "down",
];

pub const INFO_KEYWORDS: &[&str] = &[
    "what is",
    "who is",
    "where is",
    "when is",
    "how to",
    "tell me about",
    "explain",
    "define",
    "information about",
];

pub const NO_SEARCH_KEYWORDS: &[&str] = &[
    "hi",
    "hello",
    "hey",
    "thanks",
    "thank you",
    "bye",
    "goodbye",
    "please",
    "help me write",
    "code",
    "implement",
    "fix",
    "debug",
    "create",
    "make",
    "build",
];

pub fn should_use_web_search(command: &str, force_search: bool, no_search: bool) -> bool {
    if force_search {
        return true;
    }
    if no_search {
        return false;
    }

    let lower_command = command.to_lowercase();

    if NO_SEARCH_KEYWORDS
        .iter()
        .any(|&keyword| lower_command.contains(keyword))
        && !WEB_SEARCH_KEYWORDS
            .iter()
            .any(|&keyword| lower_command.contains(keyword))
    {
        return false;
    }

    if WEB_SEARCH_KEYWORDS
        .iter()
        .any(|&keyword| lower_command.contains(keyword))
    {
        return true;
    }

    if INFO_KEYWORDS
        .iter()
        .any(|&keyword| lower_command.starts_with(keyword))
    {
        return lower_command.contains("company")
            || lower_command.contains("person")
            || lower_command.contains("event")
            || lower_command.contains("place")
            || lower_command.contains("product");
    }

    false
}
