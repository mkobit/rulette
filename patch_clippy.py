with open("src/main.rs", "r") as f:
    content = f.read()

content = content.replace("""    let level = match log_level.as_deref() {
        Some(lvl) => lvl,
        None => "warn",
    };""", """    let level = log_level.as_deref().unwrap_or("warn");""")

with open("src/main.rs", "w") as f:
    f.write(content)
