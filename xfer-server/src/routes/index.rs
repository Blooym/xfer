pub async fn index_handler() -> &'static str {
    concat!(
        "xfer relay server ready for transfers.\n\n",
        env!("CARGO_PKG_REPOSITORY")
    )
}
