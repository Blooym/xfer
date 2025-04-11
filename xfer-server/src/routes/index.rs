pub async fn index_handler() -> &'static str {
    concat!(
        "xfer transfer server ready.\n\n",
        env!("CARGO_PKG_REPOSITORY")
    )
}
