use log::warn;

pub fn clean_span(span: rustc_span::Span) -> String {
    let mut span_string = format!("{:?}", span);

    if let Some(home_dir) = dirs::home_dir() {
        let home_dir_string = home_dir.to_str().unwrap();

        if !span_string.starts_with(home_dir_string) {
            let mut cwd = std::env::current_dir().unwrap();
            cwd.push(span_string);

            span_string = String::from(cwd.to_str().unwrap());
        }
    } else {
        warn!("Could not get home directory.")
    }

    span_string.trim_end_matches(" (#0)").to_owned()
}
